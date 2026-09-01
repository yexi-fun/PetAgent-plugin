# 桌宠时钟 MCP 插件

提供 `clock_now` 工具。Agent 调用后返回本地时间，并通过宿主的 `bubble.text` 结果约定，在桌宠旁的独立气泡窗口显示“现在时间：YYYY-MM-DD HH:MM:SS”。插件不访问网络、不读写文件，也不需要额外配置。

工具在 PetAgent 中的完整名称为 `com_petagent_clock__clock__clock_now`。

