# 硬件温度 native-dll 插件

该插件提供 `hardware.temperature` 工具，用于读取 CPU、GPU 和系统热区温度。

数据来源：NVIDIA GPU 优先使用 NVML（同一设备的 WMI 温度会去重）；CPU、AMD GPU、Intel GPU 等温度使用 LibreHardwareMonitor 或 OpenHardwareMonitor 的 WMI 传感器。WMI 传感器会先通过 `Sensor.Parent` 关联 `Hardware.HardwareType`，再按 NexBox 的 CPU/GPU 温度传感器名称筛选。ACPI thermal zone 会单独报告，不会伪装成 CPU 核心温度。

Windows 没有统一的 CPU 核心温度 API。没有硬件监控服务时，工具返回 `available: false` 以及 `sources` / `limitations` 说明。

```powershell
Set-Location plugins\com.petagent.hardware-temperature\src
cargo test
Set-Location ..\..\..
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-plugin.ps1 -PluginId com.petagent.hardware-temperature
```

插件使用固定 ABI v1，在 PetAgent 进程内加载，启用后需要重启宿主。native DLL 属于可信代码，不提供沙箱。
