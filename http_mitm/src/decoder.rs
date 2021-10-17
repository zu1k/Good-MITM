use crate::Error;
use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder};
use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http::header::{CONTENT_ENCODING, CONTENT_LENGTH};
use hyper::{Body, Error as HyperError, Response};
use std::{
    io,
    io::Error as IoError,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncBufRead, AsyncRead, BufReader};
use tokio_util::io::{ReaderStream, StreamReader};

struct IoStream<T: Stream<Item = Result<Bytes, HyperError>> + Unpin>(T);

impl<T: Stream<Item = Result<Bytes, HyperError>> + Unpin> Stream for IoStream<T> {
    type Item = Result<Bytes, IoError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        match futures::ready!(Pin::new(&mut self.0).poll_next(cx)) {
            Some(Ok(chunk)) => Poll::Ready(Some(Ok(chunk))),
            Some(Err(err)) => Poll::Ready(Some(Err(IoError::new(io::ErrorKind::Other, err)))),
            None => Poll::Ready(None),
        }
    }
}

enum Decoder {
    Body(Body),
    Decoder(Box<dyn AsyncRead + Send + Unpin>),
}

impl Decoder {
    pub fn decode(self, encoding: &str) -> Result<Self, Error> {
        if encoding == "identity" {
            return Ok(self);
        }

        let reader: Box<dyn AsyncBufRead + Send + Unpin> = match self {
            Decoder::Body(body) => Box::new(StreamReader::new(IoStream(body.into_stream()))),
            Decoder::Decoder(decoder) => Box::new(BufReader::new(decoder)),
        };

        let decoder: Box<dyn AsyncRead + Send + Unpin> = match encoding {
            "gzip" | "x-gzip" => Box::new(GzipDecoder::new(reader)),
            "deflate" => Box::new(ZlibDecoder::new(reader)),
            "br" => Box::new(BrotliDecoder::new(reader)),
            "zstd" => Box::new(ZstdDecoder::new(reader)),
            _ => return Err(Error::Decode),
        };

        Ok(Decoder::Decoder(decoder))
    }
}

impl From<Decoder> for Body {
    fn from(decoder: Decoder) -> Body {
        match decoder {
            Decoder::Body(body) => body,
            Decoder::Decoder(decoder) => Body::wrap_stream(ReaderStream::new(decoder)),
        }
    }
}

/// Decode the body of a response.
///
/// This will fail if either of the `content-encoding` or `content-length` headers are unable to be
/// parsed, or if one of the values specified in the `content-encoding` header is not supported.
pub fn decode_response(res: Response<Body>) -> Result<Response<Body>, Error> {
    let (mut parts, body) = res.into_parts();
    let mut encodings: Vec<String> = vec![];

    for val in parts.headers.get_all(CONTENT_ENCODING) {
        match val.to_str() {
            Ok(val) => {
                encodings.extend(val.split(',').map(|v| String::from(v.trim())));
            }
            Err(_) => return Err(Error::Decode),
        }
    }

    parts.headers.remove(CONTENT_ENCODING);

    if let Some(val) = parts.headers.remove(CONTENT_LENGTH) {
        match val.to_str() {
            Ok("0") => return Ok(Response::from_parts(parts, body)),
            Err(_) => return Err(Error::Decode),
            _ => (),
        }
    }

    let mut decoder = Decoder::Body(body);

    while let Some(encoding) = encodings.pop() {
        decoder = decoder.decode(&encoding)?;
    }

    Ok(Response::from_parts(parts, decoder.into()))
}
