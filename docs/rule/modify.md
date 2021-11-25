# 修改器

修改器用来执行修改操作，包括修改请求和修改返回

## 候选项

根据需要修改的内容的位置，修改器分为以下几类：

- Header(HeaderModify)
- Cookie(CookieModify)
- Body(BodyModify)

### Header修改

```yaml
- name: "modify response header"
  filter:
    domain-suffix: 'lgf.im'
  action:
    modify-response:
      header:
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