# Action 动作

`Action` 用来对请求或者返回进行操作

## 候选项

`Action`目前包含以下选项：

- Reject
- Redirect(String)
- ModifyRequest(Modify)
- ModifyResponse(Modify)
- LogRes
- LogReq

### Reject 拒绝

`reject`类型直接返回`502`，用来拒绝某些请求，可以用来拒绝追踪和广告

```yaml
- name: "reject CSDN"
  filter:
    domain-keyword: 'csdn'
  action: reject
```

### Redirect 重定向

`redirect`类型直接返回`302`重定向

```yaml
- name: "youtube-1"
  filter:
    url-regex: '(^https?:\/\/(?!redirector)[\w-]+\.googlevideo\.com\/(?!dclk_video_ads).+)(ctier=L)(&.+)'
  action:
    redirect: "$1$4"
```

### ModifyRequest 修改请求

`modify-request`用来修改请求，具体修改规则见 [修改器](rule/modify.md)

### ModifyResponse 修改返回

`modify-response`用来修改返回，具体修改规则见 [修改器](rule/modify.md)

### Log 记录日志

`log-req` 用来记录请求，`log-res` 用来记录返回

## 多个动作

`actions`字段支持单个动作和多个动作，当需要执行多个动作时，应使用数组

```yaml
- name: "youtube-1"
  filter:
    url-regex: '(^https?:\/\/(?!redirector)[\w-]+\.googlevideo\.com\/(?!dclk_video_ads).+)(ctier=L)(&.+)'
  actions:
    - log-req:
    - redirect: "$1$4"
```
