# Good Man in the Middle

[![GitHub stars](https://img.shields.io/github/stars/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/network)
[![Release](https://img.shields.io/github/release/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/releases)
[![GitHub issues](https://img.shields.io/github/issues/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/issues)
[![Build](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml/badge.svg)](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml)
[![GitHub license](https://img.shields.io/github/license/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/blob/master/LICENSE)
[![Docs](https://img.shields.io/badge/docs-read-blue.svg?style=flat)](https://good-mitm.zu1k.com/)

#### [English](https://github.com/zu1k/good-mitm/blob/master/README.md)

基于规则的 MITM 引擎，支持 HTTP(S) 请求和返回的重写、重定向和阻断操作，支持 JavaScript 脚本介入

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

### 证书准备

由于`MITM`技术的需要，需要你生成并信任自己的根证书

#### 生成根证书

出于安全考虑，请不要随意信任任何陌生人提供的根证书，你需要自己生成属于自己的根证书和私钥

经验丰富的用户可以自行使用OpenSSL进行相关操作，考虑到没有相关经验的用户，可以使用以下命令直接生成相关内容，生成的证书和私钥将存储在`ca`目录下

```shell
good-mitm.exe genca
```

在浏览器使用了Good-MITM提供的代理后，通过访问 [http://cert.mitm.plus](http://cert.mitm.plus) 可以直接下载证书，这在给其他设备提供服务时非常有用

#### 信任证书

你可以将根证书添加到操作系统或者浏览器的信任区中，根据你的需要自行选择

### 代理

启动Good-MITM，指定使用的规则文件或目录

```shell
good-mitm.exe run -r rules
```

在浏览器或操作系统中使用Good-MITM提供的http代理：`http://127.0.0.1:34567`

#### 透明代理

See https://docs.mitmproxy.org/stable/howto-transparent/ for docs.

```shell
sudo sysctl -w net.ipv4.ip_forward=1
sudo sysctl -w net.ipv6.conf.all.forwarding=1
sudo sysctl -w net.ipv4.conf.all.send_redirects=0

sudo useradd --create-home mitm
sudo -u mitm -H bash -c 'good-mitm run -r rules/log.yaml -b 0.0.0.0:34567'

sudo iptables -t nat -A OUTPUT -p tcp -m owner ! --uid-owner mitm --dport 80 -j REDIRECT --to-port 34567
sudo iptables -t nat -A OUTPUT -p tcp -m owner ! --uid-owner mitm --dport 443 -j REDIRECT --to-port 34567
sudo ip6tables -t nat -A OUTPUT -p tcp -m owner ! --uid-owner mitm --dport 80 -j REDIRECT --to-port 34567
sudo ip6tables -t nat -A OUTPUT -p tcp -m owner ! --uid-owner mitm --dport 443 -j REDIRECT --to-port 34567
```

## Rule 规则

`Rule`用来操控 Good-MITM

一条合格的规则需要包含以下内容:

- `规则名`：用来区分不同的规则，便与维护
- [`筛选器`](#filter)：用于从众多`请求`和`返回`中筛选出需要处理的内容
- [`动作`](#action)：用于执行想要的行为，包括`重定向`、`阻断`、`修改`等
- 必要时指定需要MITM的域名

```yaml
- name: "屏蔽Yutube追踪"
  mitm: "*.youtube.com"
  filter:
    url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
  action: reject
```

同时一条合格的规则需要符合以下要求:

- 专注：一条规则只用来做一件事
- 简单：使用简单的方法来处理，便与维护
- 高效：尽量使用高效的方法，比如使用域名后缀和域名前缀来替换域名正则表达式


### Filter 筛选器 <span id="filter"></span>

`Filter`用来筛选需要处理的请求和返回

#### 候选项

`Filter`目前包含以下类型：

- All
- Domain(String)
- DomainKeyword(String)
- DomainPrefix(String)
- DomainSuffix(String)
- UrlRegex(fancy_regex::Regex)

> **注意**  
> 当前版本中，`domain`相关类型匹配的是`host`，通常情况下不会影响结果  
> 在网站使用非常规端口时，规则需要注明端口  
> 后续版本将会对此行为进行优化  

##### All 全部

指定筛选器为`all`时将会命中全部请求和返回，通常用来执行日志记录行为

```yaml
- name: "log"
  filter: all
  action:
    - log-req
    - log-res
```

##### Domain 域名

`domain`对域名进行全量匹配

```yaml
- name: "redirect"
  filter:
    domain: 'none.zu1k.com'
  action:
    redirect: "https://zu1k.com/"
```

##### DomainKeyword 域名关键词

`domain-keyword`对域名进行关键词匹配

```yaml
- name: "reject CSDN"
  filter:
    domain-keyword: 'csdn'
  action: reject
```

##### DomainPrefix 域名前缀

`domain-prefix`对域名进行前缀匹配

```yaml
- name: "ad prefix"
  filter:
    domain-prefix: 'ads' // example: "ads.xxxxx.com"
  action: reject
```

##### DomainSuffix 域名后缀

`domain-suffix`对域名进行后缀匹配


```yaml
- name: "redirect"
  filter:
    domain-suffix: 'google.com.cn'
  action:
    redirect: "https://google.com"
```

##### UrlRegex Url正则

`url-regex`对整个url进行正则匹配

```yaml
- name: "youtube追踪"
  mitm: "*.youtube.com"
  filter:
    url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
  action: reject
```

#### 多个筛选器

`filters`字段支持单个筛选器和多个筛选器，多个筛选器之间的关系为`或`

```yaml
- name: "youtube-2"
  mitm:
    - "*.youtube.com"
    - "*.googlevideo.com"
  filters:
    - url-regex: '^https?:\/\/[\w-]+\.googlevideo\.com\/(?!(dclk_video_ads|videoplayback\?)).+(&oad|ctier)'
    - url-regex: '^https?:\/\/(www|s)\.youtube\.com\/api\/stats\/ads'
    - url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
    - url-regex: '^https?:\/\/\s.youtube.com/api/stats/qoe?.*adformat='
  action: reject
```

具有相同动作的多个规则可聚合为一个规则以便于维护

### Action 动作 <span id="action"></span>

`Action` 用来对请求或者返回进行操作

#### 候选项

`Action`目前包含以下选项：

- Reject
- Redirect(String)
- ModifyRequest(Modify)
- ModifyResponse(Modify)
- LogRes
- LogReq

##### Reject 拒绝

`reject`类型直接返回`502`，用来拒绝某些请求，可以用来拒绝追踪和广告

```yaml
- name: "reject CSDN"
  filter:
    domain-keyword: 'csdn'
  action: reject
```

##### Redirect 重定向

`redirect`类型直接返回`302`重定向

```yaml
- name: "youtube-1"
  filter:
    url-regex: '(^https?:\/\/(?!redirector)[\w-]+\.googlevideo\.com\/(?!dclk_video_ads).+)(ctier=L)(&.+)'
  action:
    redirect: "$1$4"
```

##### ModifyRequest 修改请求

`modify-request`用来修改请求，具体修改规则见 [修改器](#modify)

##### ModifyResponse 修改返回

`modify-response`用来修改返回，具体修改规则见 [修改器](#modify)

##### Log 记录日志

`log-req` 用来记录请求，`log-res` 用来记录返回

#### 多个动作

`actions`字段支持单个动作和多个动作，当需要执行多个动作时，应使用数组

```yaml
- name: "youtube-1"
  filter:
    url-regex: '(^https?:\/\/(?!redirector)[\w-]+\.googlevideo\.com\/(?!dclk_video_ads).+)(ctier=L)(&.+)'
  actions:
    - log-req:
    - redirect: "$1$4"
```

### 修改器 <span id="modify"></span>

修改器用来执行修改操作，包括修改请求和修改返回

#### 候选项

根据需要修改的内容的位置，修改器分为以下几类：

- Header(MapModify)
- Cookie(MapModify)
- Body(TextModify)

##### TextModify 文本修改器

`TextModify` 主要对文本就行修改，目前支持两种方式：

- 直接设置文本内容
- 普通替换或者正则替换

###### 直接设置

对于plain类型直接设置，内容将被直接重置为指定文本

```yaml
- name: "modify response body plain"
  filter:
    domain: '126.com'
  action:
    modify-response:
      body: "Hello 126.com, from Good-MITM"
```

###### 替换

替换支持简单替换和正则替换两种

简单替换

```yaml
- name: "modify response body replace"
  filter:
    domain-suffix: '163.com'
  action:
    modify-response:
      body:
        origin: "网易首页"
        new: "Good-MITM 首页"
```

正则替换

```yaml
- name: "modify response body regex replace"
  filter:
    domain-suffix: 'zu1k.com'
  action:
    - modify-response:
        body:
          re: '(\d{4})'
          new: 'maybe $1'

```

##### MapModify 字典修改器

`MapModify` 字典修改器主要针对字典类型的位置进行修改，例如 `header` 和 `cookies`

`key` 代表字典的键，必须指定

`value` 是 `TextModify` 类型，按照上文方法书写

如果指定 `remove` 为 `true`，则会删除该键值对

```yaml
- name: "modify response header"
  filter:
    domain: '126.com'
  action:
    - modify-response:
        header:
          key: date
          value:
            origin: "2022"
            new: "1999"
    - modify-response:
        header:
          key: new-header-item
          value: Good-MITM
    - modify-response:
        header:
          key: server
          remove: true
```

##### Header 修改

见 `MapModify` 部分方法

##### Cookie 修改

与 Header 修改方法一致

如果指定 `remove` 为 `true` 还会同时对应的移除`set-cookie`项

##### Body修改

见 `TextModify` 部分

## License

**Good-MITM** © [zu1k](https://github.com/zu1k), Released under the [MIT](./LICENSE) License.
