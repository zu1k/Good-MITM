# 证书准备

### Certificate Preparation

For MITM functionality, it is required that you trust the self-signed root certificate.

#### Generate your own root certificate

For security reasons, you need to generate your own root certificate.

```shell
good-mitm.exe genca
```

#### Trust your root certificate

You need to trust the root certificate just generated, either by adding trust in your browser or in your operating system's root certificate list, as you wish.
