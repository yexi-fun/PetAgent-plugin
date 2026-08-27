# 契约检查

- `initialize`、`health`、`capabilities`、`tools/list`、`tools/call` 和 `shutdown` 返回带匹配 id 的 JSON-RPC 2.0 响应。
- `tools/list` 只公开无参数的 `disk_usage` 工具，未知工具名返回 `-32602`。
- 工具返回每个已挂载磁盘的字节数、GiB 数和两位小数占用百分比；总容量为各磁盘汇总值。
- 容量计算使用饱和减法并处理零容量，不因异常系统值溢出或除零。
- 请求和响应使用单行 JSON，输入限制为 1 MiB；stdout 不输出日志。
- 插件不读取文件内容、不写入磁盘、不访问网络。
- 停用插件后，宿主注销 `com_petagent_disk_usage__disk__disk_usage` 并结束 stdio 子进程。
