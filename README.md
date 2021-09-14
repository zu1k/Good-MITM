# Good Man in the Middle

Use MITM technology to provide features like `rewrite`, `redirect`.

Work is still in the idea validation phase!

## Usage

### Certificate Preparation

For MITM functionality, it is required that you trust the self-signed root certificate.

#### Generate your own root certificate

For security reasons, you need to generate your own root certificate.

**DO NOT USE** the cert in the `assets/ca` directory, otherwise a security risk will lurk.

Use [examples/gen_ca.rs](examples/gen_ca.rs) to generate your own root certificate.

#### Trust your root certificate

You need to trust the root certificate just generated, either by adding trust in your browser or in your operating system's root certificate list, as you wish.

### Use the proxy provided by `good-MITM`

Adding `http` and `https` proxies to the browser, `http://127.0.0.1:34567` if not modified.

## Thanks

- [**hudsucker**](https://github.com/omjadas/hudsucker): a Rust crate providing MITM features
