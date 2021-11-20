# Good Man in the Middle

[![GitHub stars](https://img.shields.io/github/stars/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/network)
[![Release](https://img.shields.io/github/release/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/releases)
[![GitHub issues](https://img.shields.io/github/issues/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/issues)
[![Build](https://github.com/zu1k/good-mitm/actions/workflows/build.yml/badge.svg)](https://github.com/zu1k/good-mitm/actions/workflows/build.yml)
[![GitHub license](https://img.shields.io/github/license/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/blob/master/LICENSE)
[![Docs](https://img.shields.io/badge/docs-read-blue.svg?style=flat)](https://docs.mitm.plus)

利用`MITM`技术实现请求和返回的`重写`、`重定向`、`阻断`等操作

## 使用方法

这里仅介绍最基本的使用流程，具体使用方法和规则请查看[文档](https://docs.mitm.plus)

### 证书准备

由于`MITM`技术的需要，需要你生成并信任自己的根证书

#### 生成根证书

出于安全考虑，请不要随意信任任何陌生人提供的根证书，你需要自己生成属于自己的根证书和私钥

```shell
good-mitm.exe genca
```

上面命令将会生成私钥和证书，文件将存储在`ca`文件夹下

#### 信任证书

你可以将根证书添加到操作系统或者浏览器的信任区中，根据你的需要自行选择

### 代理

启动Good-MITM，指定使用的规则文件或目录

```shell
good-mitm.exe run -r rules
```

在浏览器或操作系统中使用Good-MITM提供的http代理：`http://127.0.0.1:34567`

## License

MIT License

Copyright (c) 2021 zu1k
