# 修改器

修改器用来执行修改操作，包括修改请求和修改返回

## 候选项

根据需要修改的内容的位置，修改器分为以下几类：

- Header(HeaderModify)
- Cookie
- Body(BodyModify)

> 注意  
> 其中部分功能还未完成  
> 所有修改目前仅支持字符串替换

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
  filter:
    url-regex: '(nfmovies)(?!.*?(\.css|\.js|\.jpeg|\.png|\.gif)).*'
  action:
    modify-response:
      body:
        origin: '<head>'
        new: '<link rel="stylesheet" href="https://limbopro.com/CSS/nfmovies.css" type="text/css"><script type="text javascript"  src="//limbopro.com/Adguard/nfmovies.js"></script></head>'
```
