# Good Man in the Middle

[![GitHub stars](https://img.shields.io/github/stars/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/network)
[![Release](https://img.shields.io/github/release/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/releases)
[![GitHub issues](https://img.shields.io/github/issues/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/issues)
[![Build](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml/badge.svg)](https://github.com/zu1k/good-mitm/actions/workflows/build-test.yml)
[![GitHub license](https://img.shields.io/github/license/zu1k/good-mitm)](https://github.com/zu1k/good-mitm/blob/master/LICENSE)
[![Docs](https://img.shields.io/badge/docs-read-blue.svg?style=flat)](https://good-mitm.zu1k.com/)

#### [中文版](https://github.com/zu1k/good-mitm/blob/master/README_zh.md)

Rule-based MITM engine. Rewriting, redirecting and rejecting on HTTP(S) requests and responses, supports JavaScript.

## Features

- Signing certificate automatically based on TLS ClientHello SNI extension
- Support selective MITM for specific domains
- Rule description language based on YAML format: rewrite, reject, redirect
  - Flexible rule matching capabilities
    - Domain name prefix/suffix/exact match
    - Regular expression matching
    - Multiple filter rules
  - Flexible text content rewriting
    - Erase/replace
    - Regular expression substitution
  - Flexible dictionary-based content rewriting
    - HTTP header rewriting
    - Cookie rewriting
  - Support for multiple actions per rule
- JavaScript script rules support (programmatic intervention)
- Transparent proxy support
- Support HTTPS and HTTP multiplexing on a single port
- Install CA certificate to the system trust zone

## Usage

### Certificate Preparation

Due to the requirement of the `MITM` technique, you need to generate and trust your own root certificate.

#### Generate Root Certificate

For security reasons, please do not blindly trust any root certificate provided by strangers. You need to generate your own root certificate and private key.

Experienced users can use OpenSSL to perform the necessary operations. However, for users without experience in this area, you can use the following command to generate the required content. The generated certificate and private key will be stored in the `ca` directory.

```shell
good-mitm.exe genca
```

After using the proxy provided by Good-MITM in your browser, you can directly download the certificate by visiting [http://cert.mitm.plus](http://cert.mitm.plus). This is particularly useful when providing services to other devices.

#### Trusting the Certificate

You can add the root certificate to the trust zone of your operating system or browser, depending on your needs.

### Proxy

Start Good-MITM and specify the rule file or directory to use.

```shell
good-mitm.exe run -r rules
```

Use the HTTP proxy provided by Good-MITM in your browser or operating system: `http://127.0.0.1:34567`.

#### Transparent Proxy

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

## Rule

`Rule` is used to manipulate Good-MITM.

A valid rule should include the following components:

- `Name`：Used to differentiate different rules for easier maintenance.
- [`Filter`](#filter)：Used to select the content to be processed from a set of `requests` and `responses`.
- [`Action`](#action)：Used to perform desired actions, including `redirect`, `reject`, `modification`, etc.
- Optionally, specify the domain name that requires MITM.

```yaml
- name: "Block YouTube tracking"
  mitm: "*.youtube.com"
  filter:
    url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
  action: reject
```

Additionally, a valid rule should meet the following requirements:

- Focus: Each rule should be designed to perform a single task.
- Simplicity: Use straightforward methods for processing to ensure easy maintenance.
- Efficiency: Use efficient methods whenever possible, such as using domain suffixes and prefixes instead of complex regular expressions for domain matching.

### Filter <span id="filter"></span>

`Filter`is used to select the requests and responses that need to be processed.

#### Available Options

Currently, `Filter` includes the following types:

- All
- Domain(String)
- DomainKeyword(String)
- DomainPrefix(String)
- DomainSuffix(String)
- UrlRegex(fancy_regex::Regex)

> **Note**  
> In the current version, the `domain` related types match the `host` field, which usually does not affect the results.
> If a website is using a non-standard port, the rule needs to specify the port.
> This behavior will be optimized in future versions.

##### All

When specifying the filter as `all`, it will match all requests and responses. This is typically used for performing logging actions.

```yaml
- name: "log"
  filter: all
  action:
    - log-req
    - log-res
```

##### Domain

`domain` performs a full match against the domain name.

```yaml
- name: "redirect"
  filter:
    domain: 'none.zu1k.com'
  action:
    redirect: "https://zu1k.com/"
```

##### DomainKeyword

`domain-keyword` performs a keyword match against the domain name.

```yaml
- name: "reject CSDN"
  filter:
    domain-keyword: 'csdn'
  action: reject
```

##### DomainPrefix

`domain-prefix` performs a prefix match against the domain name.

```yaml
- name: "ad prefix"
  filter:
    domain-prefix: 'ads' // example: "ads.xxxxx.com"
  action: reject
```

##### DomainSuffix

`domain-suffix` performs a suffix match against the domain name.


```yaml
- name: "redirect"
  filter:
    domain-suffix: 'google.com.cn'
  action:
    redirect: "https://google.com"
```

##### UrlRegex Url

`url-regex` performs a regular expression match against the entire URL.

```yaml
- name: "youtube tracking"
  mitm: "*.youtube.com"
  filter:
    url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
  action: reject
```

#### Multiple Filter

The `filters` field supports both single filters and multiple filters, with the relationship between multiple filters being `OR`.

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

Multiple rules with the same action can be aggregated into a single rule for easier maintenance.

### Action <span id="action"></span>

`Action` is used to perform operations on requests or responses.

#### Available Options

Currently, `Action` includes the following options:

- Reject
- Redirect(String)
- ModifyRequest(Modify)
- ModifyResponse(Modify)
- LogRes
- LogReq

##### Reject

The `reject` type directly returns `502` status code, which is used to reject certain requests. It can be used to block tracking and ads.

```yaml
- name: "reject CSDN"
  filter:
    domain-keyword: 'csdn'
  action: reject
```

##### Redirect

The `redirect` type directly returns `302` status code for redirection.

```yaml
- name: "youtube-1"
  filter:
    url-regex: '(^https?:\/\/(?!redirector)[\w-]+\.googlevideo\.com\/(?!dclk_video_ads).+)(ctier=L)(&.+)'
  action:
    redirect: "$1$4"
```

##### ModifyRequest

`modify-request` is used to modify the request. For specific modification rules, refer to the [Modify](#modify) section.

##### ModifyResponse

`modify-response` is used to modify the response. For specific modification rules, refer to the [Modify](#modify) section.

##### Log

`log-req` is used to log the request, and `log-res` is used to log the response.

#### Multiple Action

The `actions` field supports both single actions and multiple actions. When multiple actions need to be performed, an array should be used.

```yaml
- name: "youtube-1"
  filter:
    url-regex: '(^https?:\/\/(?!redirector)[\w-]+\.googlevideo\.com\/(?!dclk_video_ads).+)(ctier=L)(&.+)'
  actions:
    - log-req:
    - redirect: "$1$4"
```

### Modify <span id="modify"></span>

Modify are used to perform modification operations, including modifying requests and modifying responses.

#### Available Options

Based on the location of the content to be modified, the modifiers can be categorized as follows:

- Header(MapModify)
- Cookie(MapModify)
- Body(TextModify)

##### TextModify

`TextModify` is mainly used for modifying text. Currently, it supports two methods:

- Setting the text content directly.
- Simple replacement or regular expression replacement.

###### Setting Text Directly

For the plain type, the content will be directly set to the specified text.

```yaml
- name: "modify response body plain"
  filter:
    domain: '126.com'
  action:
    modify-response:
      body: "Hello 126.com, from Good-MITM"
```

###### Replacement

Replacement supports both simple replacement and regular expression replacement.

Simple Replacement

```yaml
- name: "modify response body replace"
  filter:
    domain-suffix: '163.com'
  action:
    modify-response:
      body:
        origin: "NetEase homepage"
        new: "Good-MITM homepage"
```

Regular expression replacement.

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

##### MapModify

`MapModify` is a modifier used to modify dictionary-type locations, such as `header` and `cookies`.

The `key` represents the key in the dictionary and must be specified.

The `value` is of type `TextModify` and follows the methods mentioned above.

If `remove` is set to `true`, the key-value pair will be removed.

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

##### Header Modification

Refer to the methods in the `MapModify` section.

##### Cookie Modification

Same as the Header modification method.

If `remove` is set to `true`, the corresponding `set-cookie` item will also be removed.

##### Body Modification

Refer to the methods in the `TextModify` section.

## License

**Good-MITM** © [zu1k](https://github.com/zu1k), Released under the [MIT](./LICENSE) License.
