# 磁盘占用 MCP 插件

该插件为 PetAgent 提供 `disk_usage` 工具，用于读取系统当前挂载磁盘的容量信息，包括挂载点、卷名、文件系统、磁盘类型、总容量、已用空间、可用空间、占用百分比和是否可移动。

插件不接受路径参数，不读取文件内容，不访问网络，也不修改磁盘。返回容量同时包含字节和 GiB 两种单位，无需额外配置。

## 本地验证

```powershell
Set-Location plugins\com.petagent.disk-usage\src
cargo test
Set-Location ..\..\..
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-plugin.ps1 -PluginId com.petagent.disk-usage
```

PetAgent 将最终工具名注册为 `com_petagent_disk_usage__disk__disk_usage`。

PetAgent 将市场插件作为可信代码运行，不提供沙箱，也不强制执行 manifest 中的权限标签。`filesystem-read` 仅用于描述该插件会查询系统磁盘元数据。
