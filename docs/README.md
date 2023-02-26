# Good Man in the Middle

[![GitHub stars](https://img.shields.io/github/stars/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/network)
[![GitHub issues](https://img.shields.io/github/issues/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/issues)
[![Build](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml/badge.svg)](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml)
[![GitHub license](https://img.shields.io/github/license/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/blob/master/LICENSE)

使用MITM技术来提供 `rewrite`、`redirect`、`reject` 等功能

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
