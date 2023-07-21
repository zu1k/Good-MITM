/// from https://github.com/branlwyd/rspd/blob/master/src/main.rs
use byteorder::{ByteOrder, NetworkEndian};
use pin_project::pin_project;
use std::{
    cmp::min,
    io::ErrorKind,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{self, AsyncRead, AsyncReadExt, AsyncWrite, Error, ReadBuf},
    pin,
};

#[pin_project]
pub struct RecordingBufReader<R: AsyncRead> {
    #[pin]
    reader: R,
    buf: Vec<u8>,
    read_offset: usize,
}

const RECORDING_READER_BUF_SIZE: usize = 1 << 10; // 1 KiB

impl<R: AsyncRead> RecordingBufReader<R> {
    pub fn new(reader: R) -> RecordingBufReader<R> {
        RecordingBufReader {
            reader,
            buf: Vec::with_capacity(RECORDING_READER_BUF_SIZE),
            read_offset: 0,
        }
    }

    pub fn buf(self) -> Vec<u8> {
        self.buf
    }
}

impl<R: AsyncRead> AsyncRead for RecordingBufReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        caller_buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), Error>> {
        // if we don't have any buffered bytes, read some bytes into our buffer.
        let mut this = self.project();
        if *this.read_offset == this.buf.len() {
            this.buf.reserve(RECORDING_READER_BUF_SIZE);
            let mut read_buf = ReadBuf::uninit(this.buf.spare_capacity_mut());
            match this.reader.as_mut().poll_read(cx, &mut read_buf) {
                Poll::Ready(Ok(())) => {
                    let bytes_read = read_buf.filled().len();
                    let new_len = this.buf.len() + bytes_read;
                    unsafe {
                        this.buf.set_len(new_len); // lol
                    }
                }
                rslt => return rslt,
            };
        }

        // read from the buffered bytes into the caller's buffer.
        let unread_bytes = &this.buf[*this.read_offset..];
        let n = min(caller_buf.remaining(), unread_bytes.len());
        caller_buf.put_slice(&unread_bytes[..n]);
        *this.read_offset += n;
        Poll::Ready(Ok(()))
    }
}

#[pin_project]
pub struct PrefixedReaderWriter<T: AsyncRead + AsyncWrite> {
    #[pin]
    inner: T,
    prefix: Vec<u8>,
    prefix_read_offset: usize,
}

impl<T: AsyncRead + AsyncWrite> PrefixedReaderWriter<T> {
    pub fn new(inner: T, prefix: Vec<u8>) -> PrefixedReaderWriter<T> {
        PrefixedReaderWriter {
            inner,
            prefix,
            prefix_read_offset: 0,
        }
    }
}

impl<T: AsyncRead + AsyncWrite> AsyncRead for PrefixedReaderWriter<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let this = self.project();
        if this.prefix.is_empty() {
            return this.inner.poll_read(cx, buf);
        }

        let prefix = &this.prefix[*this.prefix_read_offset..];
        let n = min(buf.remaining(), prefix.len());
        buf.put_slice(&prefix[..n]);
        *this.prefix_read_offset += n;

        if *this.prefix_read_offset == this.prefix.len() {
            mem::take(this.prefix);
        }

        Poll::Ready(Ok(()))
    }
}

impl<T: AsyncRead + AsyncWrite> AsyncWrite for PrefixedReaderWriter<T> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let this = self.project();
        this.inner.poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        let this = self.project();
        this.inner.poll_shutdown(cx)
    }
}

#[pin_project]
pub struct HandshakeRecordReader<R: AsyncRead> {
    #[pin]
    reader: R,
    currently_reading: HandshakeRecordReaderReading,
}

impl<R: AsyncRead> HandshakeRecordReader<R> {
    pub fn new(reader: R) -> HandshakeRecordReader<R> {
        HandshakeRecordReader {
            reader,
            currently_reading: HandshakeRecordReaderReading::ContentType,
        }
    }
}

enum HandshakeRecordReaderReading {
    ContentType,
    MajorMinorVersion(usize),
    RecordSize([u8; 2], usize),
    Record(usize),
}

impl<R: AsyncRead> AsyncRead for HandshakeRecordReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        caller_buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), Error>> {
        let mut this = self.project();
        loop {
            match this.currently_reading {
                HandshakeRecordReaderReading::ContentType => {
                    const CONTENT_TYPE_HANDSHAKE: u8 = 22;
                    let mut buf = [0];
                    let mut buf = ReadBuf::new(&mut buf[..]);
                    match this.reader.as_mut().poll_read(cx, &mut buf) {
                        Poll::Ready(Ok(())) if buf.filled().len() == 1 => {
                            if buf.filled()[0] != CONTENT_TYPE_HANDSHAKE {
                                return Poll::Ready(Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    format!(
                                        "got wrong content type (wanted {}, got {})",
                                        CONTENT_TYPE_HANDSHAKE,
                                        buf.filled()[0]
                                    ),
                                )));
                            }
                            *this.currently_reading =
                                HandshakeRecordReaderReading::MajorMinorVersion(0);
                        }
                        rslt => return rslt,
                    }
                }

                HandshakeRecordReaderReading::MajorMinorVersion(bytes_read) => {
                    let mut buf = [0, 0];
                    let mut buf = ReadBuf::new(&mut buf[..]);
                    buf.advance(*bytes_read);
                    match this.reader.as_mut().poll_read(cx, &mut buf) {
                        Poll::Ready(Ok(())) => {
                            *bytes_read = buf.filled().len();
                            if *bytes_read == 2 {
                                *this.currently_reading =
                                    HandshakeRecordReaderReading::RecordSize([0, 0], 0);
                            }
                        }
                        rslt => return rslt,
                    }
                }

                HandshakeRecordReaderReading::RecordSize(backing_array, bytes_read) => {
                    const MAX_RECORD_SIZE: usize = 1 << 14;
                    let mut buf = ReadBuf::new(&mut backing_array[..]);
                    buf.advance(*bytes_read);
                    match this.reader.as_mut().poll_read(cx, &mut buf) {
                        Poll::Ready(Ok(())) => {
                            *bytes_read = buf.filled().len();
                            if *bytes_read == 2 {
                                let record_size = u16::from_be_bytes(*backing_array).into();
                                if record_size > MAX_RECORD_SIZE {
                                    return Poll::Ready(Err(io::Error::new(
                                        io::ErrorKind::InvalidData,
                                        format!(
                                            "record too large ({} > {})",
                                            record_size, MAX_RECORD_SIZE
                                        ),
                                    )));
                                }
                                *this.currently_reading =
                                    HandshakeRecordReaderReading::Record(record_size)
                            }
                        }
                        rslt => return rslt,
                    }
                }

                HandshakeRecordReaderReading::Record(remaining_record_bytes) => {
                    // We ultimately want to read record bytes into `caller_buf`, but we need to
                    // ensure that we don't read more bytes than there are record bytes (and end
                    // up handing the caller record header bytes). So we call `caller_buf.take()`.
                    // Since `take` returns an independent `ReadBuf`, we have to update `caller_buf`
                    // once we're done reading: first we call `assume_init` to assert that the
                    // `bytes_read` bytes we read are initialized, then we call `advance` to assert
                    // that the appropriate section of the buffer is filled.

                    let mut buf = caller_buf.take(*remaining_record_bytes);
                    let rslt = this.reader.as_mut().poll_read(cx, &mut buf);
                    if let Poll::Ready(Ok(())) = rslt {
                        let bytes_read = buf.filled().len();
                        unsafe {
                            caller_buf.assume_init(bytes_read);
                        }
                        caller_buf.advance(bytes_read);
                        *remaining_record_bytes -= bytes_read;
                        if *remaining_record_bytes == 0 {
                            *this.currently_reading = HandshakeRecordReaderReading::ContentType;
                        }
                    }
                    return rslt;
                }
            }
        }
    }
}

pub async fn read_sni_host_name_from_client_hello<R: AsyncRead>(
    mut reader: Pin<&mut R>,
) -> io::Result<String> {
    // Handshake message type.
    const HANDSHAKE_TYPE_CLIENT_HELLO: u8 = 1;
    let typ = reader.read_u8().await?;
    if typ != HANDSHAKE_TYPE_CLIENT_HELLO {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "handshake message not a ClientHello (type {}, expected {})",
                typ, HANDSHAKE_TYPE_CLIENT_HELLO
            ),
        ));
    }

    // Handshake message length.
    let len = read_u24(reader.as_mut()).await?;
    let reader = reader.take(len.into());
    pin!(reader);

    // ProtocolVersion (2 bytes) & random (32 bytes).
    skip(reader.as_mut(), 34).await?;

    // Session ID (u8-length vec), cipher suites (u16-length vec), compression methods (u8-length vec).
    skip_vec_u8(reader.as_mut()).await?;
    skip_vec_u16(reader.as_mut()).await?;
    skip_vec_u8(reader.as_mut()).await?;

    // Extensions.
    let ext_len = reader.read_u16().await?;
    let new_limit = min(reader.limit(), ext_len.into());
    reader.set_limit(new_limit);
    loop {
        // Extension type & length.
        let ext_typ = reader.read_u16().await?;
        let ext_len = reader.read_u16().await?;

        const EXTENSION_TYPE_SNI: u16 = 0;
        if ext_typ != EXTENSION_TYPE_SNI {
            skip(reader.as_mut(), ext_len.into()).await?;
            continue;
        }
        let new_limit = min(reader.limit(), ext_len.into());
        reader.set_limit(new_limit);

        // ServerNameList length.
        let snl_len = reader.read_u16().await?;
        let new_limit = min(reader.limit(), snl_len.into());
        reader.set_limit(new_limit);

        // ServerNameList.
        loop {
            // NameType & length.
            let name_typ = reader.read_u8().await?;

            const NAME_TYPE_HOST_NAME: u8 = 0;
            if name_typ != NAME_TYPE_HOST_NAME {
                skip_vec_u16(reader.as_mut()).await?;
                continue;
            }

            let name_len = reader.read_u16().await?;
            let new_limit = min(reader.limit(), name_len.into());
            reader.set_limit(new_limit);
            let mut name_buf = vec![0; name_len.into()];
            reader.read_exact(&mut name_buf).await?;
            return String::from_utf8(name_buf)
                .map_err(|err| io::Error::new(ErrorKind::InvalidData, err));
        }
    }
}

async fn skip<R: AsyncRead>(reader: Pin<&mut R>, len: u64) -> io::Result<()> {
    let bytes_read = io::copy(&mut reader.take(len), &mut io::sink()).await?;
    if bytes_read < len {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            format!("skip read {} < {} bytes", bytes_read, len),
        ));
    }
    Ok(())
}

async fn skip_vec_u8<R: AsyncRead>(mut reader: Pin<&mut R>) -> io::Result<()> {
    let sz = reader.read_u8().await?;
    skip(reader.as_mut(), sz.into()).await
}

async fn skip_vec_u16<R: AsyncRead>(mut reader: Pin<&mut R>) -> io::Result<()> {
    let sz = reader.read_u16().await?;
    skip(reader.as_mut(), sz.into()).await
}

async fn read_u24<R: AsyncRead>(mut reader: Pin<&mut R>) -> io::Result<u32> {
    let mut buf = [0; 3];
    reader
        .as_mut()
        .read_exact(&mut buf)
        .await
        .map(|_| NetworkEndian::read_u24(&buf))
}
