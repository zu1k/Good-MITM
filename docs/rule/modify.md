# 修改器

修改器用来执行修改操作，包括修改请求和修改返回

## 候选项

根据需要修改的内容的位置，修改器分为以下几类：

- Header(MapModify)
- Cookie(MapModify)
- Body(TextModify)

### TextModify 文本修改器

`TextModify` 主要对文本就行修改，目前支持两种方式：

- 直接设置文本内容
- 普通替换或者正则替换

#### 直接设置

对于plain类型直接设置，内容将被直接重置为指定文本

```yaml
- name: "modify response body plain"
  filter:
    domain: '126.com'
  action:
    modify-response:
      body: "Hello 126.com, from Good-MITM"
```

#### 替换

替换支持简单替换和正则替换两种

##### 简单替换

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

##### 正则替换

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

### MapModify 字典修改器

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

### Header 修改

见 `MapModify` 部分方法

### Cookie 修改

与 Header 修改方法一致

如果指定 `remove` 为 `true` 还会同时对应的移除`set-cookie`项

### Body修改

见 `TextModify` 部分
