# 桌宠时钟 app 插件

受控独立桌面应用示例，由 Rust service 和独立 Vue 3 frontend 组成。service 从 Windows 系统时钟读取本地时间和 UTC offset，不访问网络、不读取文件，也不需要管理员权限。时区名称采用系统当前 offset；首版不包含 IANA 时区数据库。

Agent 工具为 `com_petagent_clock__clock_now`、`com_petagent_clock__clock_show`、`com_petagent_clock__clock_hide` 和 `com_petagent_clock__clock_set_format`。`clock.now` / `clock.show` 的结果会请求宿主打开窗口，`clock.hide` 请求关闭窗口；宿主只接受受限的 `window.open` / `window.close` 动作。frontend 订阅所属 app session 的 `clock.updated` 与 `clock.visibility` 事件，在加载后通过 `app.invoke` 获取一次初始时间，并提供窗口内关闭按钮；断开 service 后显示“暂不可用”。
