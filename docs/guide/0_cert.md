# 证书准备

为了实现对HTTPS流量进行MITM，同时为了浏览器等不显示安全警告，需要生成并信任自签名CA证书

## 生成CA证书

处于安全考虑，用户必须自己生成自己的CA证书，随意使用不可信的CA证书将会留下严重的安全隐患

经验丰富的用户可以自行使用OpenSSL进行相关操作，考虑到没有相关经验的用户，可以使用以下命令直接生成相关内容，生成的证书和私钥将存储在`ca`目录下

```shell
good-mitm.exe genca
```

## 信任生成的证书

You need to trust the root certificate just generated, either by adding trust in your browser or in your operating system's root certificate list, as you wish.
