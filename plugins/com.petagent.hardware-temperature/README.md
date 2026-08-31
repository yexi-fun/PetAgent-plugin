# 硬件温度 native-dll 插件

该插件提供 `hardware.temperature` 工具，用于读取 CPU、GPU 和系统热区温度。

数据来源：插件内置 LibreHardwareMonitor 采集 helper 和 PawnIO 安装包，不需要用户另行下载 LibreHardwareMonitor。NVIDIA GPU 优先使用 NVML（同一设备的 helper/WMI 温度会去重）；CPU、AMD GPU、Intel GPU 等温度由 helper 通过 LibreHardwareMonitorLib 采集，OpenHardwareMonitor WMI 作为兼容兜底。ACPI thermal zone 会单独报告，不会伪装成 CPU 核心温度。

首次采集会启动随插件分发的 helper；如果系统没有 PawnIO，helper 会请求一次管理员权限安装内置 `PawnIO_setup.exe`。Windows 没有统一的 CPU 核心温度 API；驱动或硬件不支持时，工具返回 `available: false` 以及 `sources` / `limitations` 说明。

```powershell
Set-Location plugins\com.petagent.hardware-temperature\src
cargo test
Set-Location ..\..\..
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-plugin.ps1 -PluginId com.petagent.hardware-temperature
```

插件使用固定 ABI v1，在 PetAgent 进程内加载，启用后需要重启宿主。native DLL 属于可信代码，不提供沙箱。
