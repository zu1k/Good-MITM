# Good Man in the Middle

[![GitHub stars](https://img.shields.io/github/stars/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/network)
[![Release](https://img.shields.io/github/release/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/releases)
[![GitHub issues](https://img.shields.io/github/issues/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/issues)
[![Build](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml/badge.svg)](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml)
[![GitHub license](https://img.shields.io/github/license/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/blob/master/LICENSE)
[![Docs](https://img.shields.io/badge/docs-read-blue.svg?style=flat)](https://good-mitm.zu1k.com/)

利用`MITM`技术实现请求和返回的`重写`、`重定向`、`阻断`等操作

## 功能

- 基于 TLS ClientHello 的自动证书签署
- 支持选择性 MITM
- 基于 YAML 格式的规则描述语言：重写/阻断/重定向
  - 灵活的规则匹配器
    - 域名前缀/后缀/全匹配
    - 正则匹配
    - 多筛选器规则
  - 灵活的文本内容改写
    - 抹除/替换
    - 正则替换
  - 灵活的字典类型内容改写
    - HTTP Header 改写
    - Cookie 改写
  - 支持单条规则多个行为
- 支持 JavaScript 脚本规则 (编程介入)
- 支持透明代理
- 透明代理 HTTPS 和 HTTP 复用单端口
- 支持自动安装 CA 证书到系统信任区

## 使用方法

这里仅介绍最基本的使用流程，具体使用方法和规则请查看[文档](https://good-mitm.zu1k.com/)

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

**Good-MITM** © [zu1k](https://github.com/zu1k), Released under the [MIT](./LICENSE) License.
