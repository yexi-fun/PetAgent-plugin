# 桌宠时钟 app 插件

受控独立桌面应用示例，由 Rust service 和独立 Vue 3 frontend 组成。service 从 Windows 系统时钟读取本地时间和 UTC offset，不访问网络、不读取文件，也不需要管理员权限。时区名称采用系统当前 offset；首版不包含 IANA 时区数据库。

Agent 工具为 `com_petagent_clock__clock_now`、`com_petagent_clock__clock_show`、`com_petagent_clock__clock_hide` 和 `com_petagent_clock__clock_set_format`。窗口由 PetAgent 创建，frontend 只接收所属 app session 的 `clock.updated` 与 `clock.visibility` 事件；断开 service 后显示“暂不可用”。
