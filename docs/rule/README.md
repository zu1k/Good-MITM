# Rule 规则

`Rule`用来操控 Good-MITM

一条合格的规则需要包含以下内容:

- `规则名`：用来区分不同的规则，便与维护
- [`筛选器`](rule/filter.md)：用于从众多`请求`和`返回`中筛选出需要处理的内容
- [`动作`](rule/action.md)：用于执行想要的行为，包括`重定向`、`阻断`、`修改`等

```yaml
- name: "屏蔽Yutube追踪"
  filter:
    url-regex: '^https?:\/\/(www|s)\.youtube\.com\/(pagead|ptracking)'
  action: reject
```

同时一条合格的规则需要符合以下要求:

- 专注：一条规则只用来做一件事
- 简单：使用简单的方法来处理，便与维护
- 高效：尽量使用高效的方法，比如使用域名后缀和域名前缀来替换域名正则表达式
