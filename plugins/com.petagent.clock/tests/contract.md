# 契约检查

- MCP 生命周期 `initialize`、`health`、`capabilities`、`tools/list`、`tools/call`、`shutdown` 均返回 JSON-RPC 2.0 响应。
- `tools/list` 公开无参数的 `clock_now` 工具。
- `clock_now` 返回本地时间字符串，并附带 `bubble.text` 供宿主独立气泡显示。
- 请求使用逐行 JSON，输入限制为 1 MiB，stdout 不输出日志。
- 停用插件后，宿主注销 `com_petagent_clock__clock__clock_now` 并结束 stdio 子进程。

