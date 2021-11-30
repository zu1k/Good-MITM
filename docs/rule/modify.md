# 修改器

修改器用来执行修改操作，包括修改请求和修改返回

## 候选项

根据需要修改的内容的位置，修改器分为以下几类：

- Header(TextModify)
- Cookie(CookieModify)
- Body(TextModify)

### TextModify

可以看到Header和Body修改目前都是TextModify类型，目前还仅支持这种方式，其他修改方式仍待开发

`TextModify`主要对文本就行修改，目前支持两种方式：

- `plain`: 普通的替换
- `regex`: 正则替换

#### plain

对于plain类型的替换，如果origin内容为空，内容将被直接重置为new的内容

```yaml
- name: "modify response body plain"
  filters: all
  actions:
    modify-response:
      body:
        type: plain
        origin: '1234'
        new: '5678'
```

#### regex

```yaml
- name: "modify response body regex"
  filters: all
  actions:
    modify-response:
      body:
        type: regex
        origin: '(\d{4})'
        new: 'number: $1'
```

### Header修改

```yaml
- name: "modify response header"
  filter:
    domain-suffix: 'lgf.im'
  action:
    modify-response:
      header:
        type: plain
        origin: "2021"
        new: "2022"
```

### Body修改

```yaml
- name: "奈菲影视去广告"
  mitm: "*nfmovies*"
  filters:
    url-regex: '(nfmovies)(?!.*?(\.css|\.js|\.jpeg|\.png|\.gif)).*'
  actions:
    modify-response:
      body:
        type: plain
        origin: '<head>'
        new: '<link rel="stylesheet" href="https://limbopro.com/CSS/nfmovies.css"......'
```

### Cookie修改

一个Cookie修改项包括3部分内容：

- `name`: 必须
- `value`: 可选，修改对应的cookie值为此值，`value`和下面的`remove`必须二选一
- `remove`: 可选，true代表移除该cookie项，在修改返回中还会同时对应的移除`set-cookie`项


```yaml
- name: "netflix-cookie"
  mitm: "*netflix.com"
  filter:
    - domain-keyword: "netflix"
  action:
    - modify-request:
        cookies:
          - name: "SecureNetflixId"
            value: "v%3D2%26mac%3DAQEAEQABABRiIX7pSoMK5G63rb8i4HAjsQ10......"
          - name: "flwssn"
            value: "febad29a-525f-4826-adf0-5e2d94011f72"
          - name: "nfvdid"
            value: "BQFmAAEBEEQy-bkd94fIPbMYcvG6Bcxg_4deRrSnzaJFMK1vmunodPN......"
          - name: "NetflixId"
            value: "v%3D2%26ct%3DBQAOAAEBEDhNm4B4vl_L8vBhlDhpUqqB0Iti......"
    - modify-response:
        cookies:
          - name: "SecureNetflixId"
            remove: true
          - name: "flwssn"
            remove: true
          - name: "nfvdid"
            remove: true
          - name: "NetflixId"
            remove: true

```