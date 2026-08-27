# 契约检查

- DLL 导出 ABI v1 的五个函数，并通过宿主架构、初始化和 health 预检。
- `capabilities` 注册 `hardware.temperature`，输入 Schema 为无参数对象。
- `invoke` 返回 CPU、GPU、ACPI thermal zone 读数及来源，不把 thermal zone 标记为 CPU。
- NVIDIA 温度来自 NVML；CPU 和 AMD/Intel GPU 温度来自 LibreHardwareMonitor/OpenHardwareMonitor WMI。
- 缺少硬件监控服务时返回 `available: false` 和可诊断的 `sources` / `limitations`。
- 每个温度限制在 -100 至 200 摄氏度，输入输出均不超过 1 MiB。
- 插件不写入磁盘、不访问网络，不接受任意路径或命令参数。
