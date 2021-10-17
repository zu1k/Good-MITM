# Filter 筛选器

`Filter`用来筛选需要处理的请求和返回

## 候选项

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

### All 全部

指定筛选器为`all`时将会命中全部请求和返回，通常用来执行日志记录行为

```yaml
- name: "log"
  filter: all
  action:
    - log-req
    - log-res
```

### Domain 域名

`domain`对域名进行全量匹配

```yaml
- name: "redirect"
  filter:
    domain: 'none.lgf.im'
  action:
    redirect: "https://lgf.im/"
```

### DomainKeyword 域名关键词

`domain-keyword`对域名进行关键词匹配

```yaml
- name: "reject CSDN"
  filter:
    domain-keyword: 'csdn'
  action: reject
```

### DomainPrefix 域名前缀

`domain-prefix`对域名进行前缀匹配

```yaml
- name: "ad prefix"
  filter:
    domain-prefix: 'ads' // example: "ads.xxxxx.com"
  action: reject
```

### DomainSuffix 域名后缀

`domain-suffix`对域名进行后缀匹配


```yaml
- name: "redirect"
  filter:
    domain-suffix: 'google.com.cn'
  action:
    redirect: "https://google.com"
```

### UrlRegex Url正则

`url-regex`对整个url进行正则匹配

```yaml
- name: "youtube追踪"
  filter:
    url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
  action: reject
```

## 多个筛选器

`filters`字段支持单个筛选器和多个筛选器，多个筛选器之间的关系为`或`

```yaml
- name: "youtube-2"
  filters:
    - url-regex: '^https?:\/\/[\w-]+\.googlevideo\.com\/(?!(dclk_video_ads|videoplayback\?)).+(&oad|ctier)'
    - url-regex: '^https?:\/\/(www|s)\.youtube\.com\/api\/stats\/ads'
    - url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
    - url-regex: '^https?:\/\/\s.youtube.com/api/stats/qoe?.*adformat='
  action: reject
```

具有相同动作的多个规则可聚合为一个规则以便于维护
