use serde_json::{json, Value};
use std::ffi::c_void;

#[repr(C)]
pub struct HostApi {
    pub struct_size: u32,
    pub api_version: u32,
}
#[repr(C)]
pub struct Buffer {
    pub data: *mut u8,
    pub len: u64,
}

const MAX_BUFFER_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
struct Reading {
    sensor_class: &'static str,
    name: String,
    value_celsius: f64,
    source: &'static str,
}

fn round_two(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn normalize_sensor_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "temperature" | "temp" => Some("temperature"),
        _ => None,
    }
}

fn is_cpu_sensor_name(name: &str) -> bool {
    let name = name.trim().to_ascii_lowercase();
    matches!(
        name.as_str(),
        "core (tctl/tdie)"
            | "cpu package"
            | "tctl"
            | "core"
            | "core max"
            | "core average"
            | "cpu"
            | "cpu core"
            | "cpu temperature"
    ) || name.starts_with("p-core #")
        || name.starts_with("e-core #")
}

fn is_gpu_sensor_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "gpu core" | "gpu" | "core" | "gpu temperature"
    )
}

fn classify_hardware(
    hardware_type: &str,
    hardware_name: &str,
    sensor_name: &str,
) -> Option<&'static str> {
    let hardware_type = hardware_type.trim().to_ascii_lowercase();
    let hardware_name = hardware_name.trim().to_ascii_lowercase();
    if hardware_type.starts_with("gpu")
        || hardware_name.contains("gpu")
        || hardware_name.contains("graphics")
        || hardware_name.contains("radeon")
        || hardware_name.contains("nvidia")
    {
        return is_gpu_sensor_name(sensor_name).then_some("gpu");
    }
    if hardware_type == "cpu"
        || hardware_name.contains("cpu")
        || hardware_name.contains("processor")
    {
        return is_cpu_sensor_name(sensor_name).then_some("cpu");
    }
    // LibreHardwareMonitor reports some older AMD CPU sensors through these
    // motherboard controllers, matching NexBox's fallback path.
    if (hardware_type == "superio" || hardware_type == "motherboard")
        && is_cpu_sensor_name(sensor_name)
    {
        return Some("cpu");
    }
    None
}

fn classify_sensor(name: &str, parent: &str, identifier: &str) -> Option<&'static str> {
    let text = format!("{name} {parent} {identifier}").to_ascii_lowercase();
    if is_gpu_sensor_name(name)
        && (text.contains("gpu")
            || text.contains("graphics")
            || text.contains("radeon")
            || text.contains("nvidia"))
    {
        Some("gpu")
    } else if is_cpu_sensor_name(name)
        && (text.contains("cpu") || text.contains("package") || text.contains("processor"))
    {
        Some("cpu")
    } else {
        None
    }
}

#[allow(dead_code)]
fn value_to_celsius(value: &str) -> Option<f64> {
    let parsed = value.trim().parse::<f64>().ok()?;
    (-100.0..200.0).contains(&parsed).then(|| round_two(parsed))
}

#[cfg(windows)]
fn variant_to_celsius(value: &wmi::Variant) -> Option<f64> {
    let parsed = match value {
        wmi::Variant::R4(value) => *value as f64,
        wmi::Variant::R8(value) => *value,
        wmi::Variant::I1(value) => *value as f64,
        wmi::Variant::I2(value) => *value as f64,
        wmi::Variant::I4(value) => *value as f64,
        wmi::Variant::I8(value) => *value as f64,
        wmi::Variant::UI1(value) => *value as f64,
        wmi::Variant::UI2(value) => *value as f64,
        wmi::Variant::UI4(value) => *value as f64,
        wmi::Variant::UI8(value) => *value as f64,
        wmi::Variant::String(value) => value.parse::<f64>().ok()?,
        _ => return None,
    };
    (-100.0..200.0).contains(&parsed).then(|| round_two(parsed))
}

#[cfg(windows)]
fn collect_lhm_helper(
    readings: &mut Vec<Reading>,
    sources: &mut Vec<&'static str>,
    nvidia_nvml_found: bool,
) {
    use std::io::{Read, Write};
    use std::net::{TcpStream, ToSocketAddrs};
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

    const HELPER_PORT: u16 = 47631;

    // Resolve the DLL's own directory instead of relying on the host cwd.
    // Plugins are extracted under a versioned directory by PetAgent.
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleExW(flags: u32, address: *const u16, module: *mut usize) -> i32;
        fn GetModuleFileNameW(module: usize, filename: *mut u16, size: u32) -> u32;
    }
    let mut module = 0usize;
    let mut path = [0u16; 32768];
    let ok = unsafe {
        GetModuleHandleExW(0x00000004, collect_lhm_helper as *const u16, &mut module) != 0
    };
    if !ok {
        sources.push("librehardwaremonitor-helper-unavailable");
        return;
    }
    let length =
        unsafe { GetModuleFileNameW(module, path.as_mut_ptr(), path.len() as u32) } as usize;
    if length == 0 || length >= path.len() {
        sources.push("librehardwaremonitor-helper-unavailable");
        return;
    }
    let module_path = std::ffi::OsString::from_wide(&path[..length]);
    let Some(module_dir) = PathBuf::from(module_path).parent().map(|p| p.to_path_buf()) else {
        sources.push("librehardwaremonitor-helper-unavailable");
        return;
    };
    let helper = module_dir.join("hardware-temperature-helper.exe");
    if !helper.is_file() {
        sources.push("librehardwaremonitor-helper-unavailable");
        return;
    }

    fn connect() -> Option<std::net::TcpStream> {
        ("127.0.0.1", HELPER_PORT)
            .to_socket_addrs()
            .ok()?
            .find_map(|address| {
                TcpStream::connect_timeout(&address, Duration::from_millis(250)).ok()
            })
    }

    // ShellExecute("runas") is required because an elevated process cannot
    // reliably inherit the native DLL's stdin/stdout pipes. The helper stays
    // resident and serves subsequent requests over loopback TCP.
    fn launch_elevated(helper: &std::path::Path, directory: &std::path::Path) -> bool {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "shell32")]
        extern "system" {
            fn ShellExecuteW(
                hwnd: *mut c_void,
                operation: *const u16,
                file: *const u16,
                parameters: *const u16,
                directory: *const u16,
                show: i32,
            ) -> isize;
        }
        let operation: Vec<u16> = std::ffi::OsStr::new("runas")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let file: Vec<u16> = helper.as_os_str().encode_wide().chain(Some(0)).collect();
        let parameters: Vec<u16> = std::ffi::OsStr::new("--server --port 47631")
            .encode_wide()
            .chain(Some(0))
            .collect();
        let directory: Vec<u16> = directory.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                parameters.as_ptr(),
                directory.as_ptr(),
                0,
            ) > 32
        }
    }

    let mut stream = connect();
    if stream.is_none() {
        if !launch_elevated(&helper, &module_dir) {
            sources.push("librehardwaremonitor-helper-elevation-denied");
            return;
        }
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(250));
            if let Some(candidate) = connect() {
                stream = Some(candidate);
                break;
            }
        }
    }
    let Some(mut stream) = stream else {
        sources.push("librehardwaremonitor-helper-unavailable");
        return;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = stream.write_all(b"read\n");
    let mut output = Vec::new();
    let _ = stream.read_to_end(&mut output);
    let Ok(response) = serde_json::from_slice::<Value>(&output) else {
        sources.push("librehardwaremonitor-helper-invalid-response");
        return;
    };
    if response.get("ok").and_then(Value::as_bool) != Some(true) {
        sources.push("librehardwaremonitor-helper-error");
        return;
    }
    let mut found = false;
    if let Some(sensors) = response.get("sensors").and_then(Value::as_array) {
        for sensor in sensors {
            if normalize_sensor_type(
                sensor
                    .get("sensorType")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ) != Some("temperature")
            {
                continue;
            }
            let name = sensor
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let hardware = sensor
                .get("hardware")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let hardware_type = sensor
                .get("hardwareType")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let Some(sensor_class) = classify_hardware(hardware_type, hardware, name)
                .or_else(|| classify_sensor(name, hardware, hardware_type))
            else {
                continue;
            };
            if nvidia_nvml_found
                && sensor_class == "gpu"
                && format!("{hardware} {hardware_type}")
                    .to_ascii_lowercase()
                    .contains("nvidia")
            {
                continue;
            }
            let Some(value) = sensor.get("value").and_then(Value::as_f64) else {
                continue;
            };
            if !(-100.0..200.0).contains(&value) {
                continue;
            }
            readings.push(Reading {
                sensor_class,
                name: name.to_string(),
                value_celsius: round_two(value),
                source: "librehardwaremonitor-helper",
            });
            found = true;
        }
    }
    sources.push(if found {
        "librehardwaremonitor-helper"
    } else {
        "librehardwaremonitor-helper-no-temperature"
    });
}

#[cfg(not(windows))]
fn collect_lhm_helper(
    _readings: &mut Vec<Reading>,
    sources: &mut Vec<&'static str>,
    _nvidia_nvml_found: bool,
) {
    sources.push("librehardwaremonitor-helper-windows-only");
}

#[cfg(windows)]
fn collect_nvml(readings: &mut Vec<Reading>, sources: &mut Vec<&'static str>) -> bool {
    use nvml_wrapper::{enum_wrappers::device::TemperatureSensor, Nvml};
    let Ok(nvml) = Nvml::init() else {
        sources.push("nvml-unavailable");
        return false;
    };
    let Ok(count) = nvml.device_count() else {
        sources.push("nvml-unavailable");
        return false;
    };
    for index in 0..count {
        let Ok(device) = nvml.device_by_index(index) else {
            continue;
        };
        let Ok(temperature) = device.temperature(TemperatureSensor::Gpu) else {
            continue;
        };
        let name = device
            .name()
            .unwrap_or_else(|_| format!("NVIDIA GPU {index}"));
        readings.push(Reading {
            sensor_class: "gpu",
            name,
            value_celsius: temperature as f64,
            source: "nvml",
        });
    }
    let found = readings.iter().any(|reading| reading.source == "nvml");
    if found {
        sources.push("nvml");
    } else {
        sources.push("nvml-no-temperature");
    }
    found
}

#[cfg(not(windows))]
fn collect_nvml(_readings: &mut Vec<Reading>, sources: &mut Vec<&'static str>) -> bool {
    sources.push("nvml-windows-only");
    false
}

#[cfg(windows)]
fn collect_hardware_monitor(
    readings: &mut Vec<Reading>,
    sources: &mut Vec<&'static str>,
    nvidia_nvml_found: bool,
) {
    use std::collections::HashMap;
    use wmi::{Variant, WMIConnection};
    let mut found_monitor = false;
    for namespace in ["ROOT\\LibreHardwareMonitor", "ROOT\\OpenHardwareMonitor"] {
        let Ok(connection) = WMIConnection::with_namespace_path(namespace) else {
            continue;
        };
        // The Sensor.Parent field is an identifier. Resolve it to Hardware so
        // we can apply the same CPU/GPU type and sensor-name filters as NexBox.
        let mut hardware = HashMap::<String, (String, String)>::new();
        if let Ok(rows) = connection.raw_query::<HashMap<String, Variant>>(
            "SELECT Identifier, Name, HardwareType FROM Hardware",
        ) {
            for row in rows {
                let string_value = |key: &str| match row.get(key) {
                    Some(Variant::String(value)) => value.clone(),
                    _ => String::new(),
                };
                let identifier = string_value("Identifier");
                let name = string_value("Name");
                if !identifier.is_empty() {
                    hardware.insert(
                        identifier.to_ascii_lowercase(),
                        (name.clone(), string_value("HardwareType")),
                    );
                }
                // Some OpenHardwareMonitor builds expose Parent as the
                // display name rather than the identifier.
                if !name.is_empty() {
                    hardware.insert(
                        name.to_ascii_lowercase(),
                        (name, string_value("HardwareType")),
                    );
                }
            }
        }
        let Ok(rows) = connection.raw_query::<HashMap<String, Variant>>(
            "SELECT Name, SensorType, Value, Parent, Identifier FROM Sensor",
        ) else {
            continue;
        };
        found_monitor = true;
        for row in rows {
            let string_value = |key: &str| match row.get(key) {
                Some(Variant::String(value)) => value.clone(),
                Some(value) => format!("{value:?}"),
                None => String::new(),
            };
            if normalize_sensor_type(&string_value("SensorType")) != Some("temperature") {
                continue;
            }
            let name = string_value("Name");
            let parent = string_value("Parent");
            let identifier = string_value("Identifier");
            let metadata = hardware
                .get(&parent.to_ascii_lowercase())
                .or_else(|| hardware.get(&identifier.to_ascii_lowercase()));
            let sensor_class = metadata
                .and_then(|(hardware_name, hardware_type)| {
                    classify_hardware(hardware_type, hardware_name, &name)
                })
                .or_else(|| classify_sensor(&name, &parent, &identifier));
            let Some(sensor_class) = sensor_class else {
                continue;
            };
            // NVML is NexBox's authoritative NVIDIA path. Keep WMI data for
            // other GPUs, but avoid duplicate NVIDIA readings when both APIs
            // expose the same device.
            let nvidia_hardware = metadata
                .map(|(hardware_name, hardware_type)| {
                    let text = format!("{hardware_name} {hardware_type}").to_ascii_lowercase();
                    text.contains("nvidia") || text.contains("gpunvidia")
                })
                .unwrap_or(false);
            if nvidia_nvml_found
                && sensor_class == "gpu"
                && (nvidia_hardware
                    || format!("{name} {parent} {identifier}")
                        .to_ascii_lowercase()
                        .contains("nvidia"))
            {
                continue;
            }
            let Some(value_celsius) = row.get("Value").and_then(variant_to_celsius) else {
                continue;
            };
            readings.push(Reading {
                sensor_class,
                name,
                value_celsius,
                source: if namespace.contains("Libre") {
                    "librehardwaremonitor-wmi"
                } else {
                    "openhardwaremonitor-wmi"
                },
            });
        }
    }
    if !found_monitor {
        sources.push("hardware-monitor-wmi-unavailable");
    }
}

#[cfg(not(windows))]
fn collect_hardware_monitor(
    _readings: &mut Vec<Reading>,
    sources: &mut Vec<&'static str>,
    _nvidia_nvml_found: bool,
) {
    sources.push("hardware-monitor-windows-only");
}

#[cfg(windows)]
fn collect_acpi_zones(readings: &mut Vec<Reading>) {
    use std::collections::HashMap;
    use wmi::{Variant, WMIConnection};
    let Ok(connection) = WMIConnection::with_namespace_path("ROOT\\WMI") else {
        return;
    };
    let Ok(rows) = connection.raw_query::<HashMap<String, Variant>>(
        "SELECT InstanceName, CurrentTemperature FROM MSAcpi_ThermalZoneTemperature",
    ) else {
        return;
    };
    for row in rows {
        let name = match row.get("InstanceName") {
            Some(Variant::String(value)) => value.clone(),
            _ => "ACPI thermal zone".to_string(),
        };
        let value = match row.get("CurrentTemperature") {
            Some(Variant::UI4(value)) => *value as f64 / 10.0 - 273.15,
            Some(Variant::I4(value)) => *value as f64 / 10.0 - 273.15,
            _ => continue,
        };
        if (-100.0..200.0).contains(&value) {
            readings.push(Reading {
                sensor_class: "thermal_zone",
                name,
                value_celsius: round_two(value),
                source: "acpi-thermal-zone",
            });
        }
    }
}

#[cfg(not(windows))]
fn collect_acpi_zones(_readings: &mut Vec<Reading>) {}

fn collect_temperatures() -> Value {
    let mut readings = Vec::new();
    let mut sources = Vec::new();
    let nvidia_nvml_found = collect_nvml(&mut readings, &mut sources);
    collect_lhm_helper(&mut readings, &mut sources, nvidia_nvml_found);
    collect_hardware_monitor(&mut readings, &mut sources, nvidia_nvml_found);
    collect_acpi_zones(&mut readings);
    readings.sort_by(|left, right| {
        left.sensor_class
            .cmp(right.sensor_class)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.source.cmp(right.source))
    });
    let cpu_count = readings
        .iter()
        .filter(|reading| reading.sensor_class == "cpu")
        .count();
    let gpu_count = readings
        .iter()
        .filter(|reading| reading.sensor_class == "gpu")
        .count();
    let output: Vec<Value> = readings.iter().map(|reading| json!({ "class": reading.sensor_class, "name": reading.name, "temperatureCelsius": reading.value_celsius, "source": reading.source })).collect();
    json!({
        "available": cpu_count > 0 || gpu_count > 0,
        "cpuTemperatureAvailable": cpu_count > 0,
        "gpuTemperatureAvailable": gpu_count > 0,
        "cpuSensorCount": cpu_count,
        "gpuSensorCount": gpu_count,
        "sensorCount": output.len(),
        "readings": output,
        "sources": sources,
        "limitations": ["Windows does not expose a universal CPU core temperature API.", "CPU and non-NVIDIA GPU readings require LibreHardwareMonitor or OpenHardwareMonitor WMI sensors.", "ACPI thermal zones are reported separately and are not labeled as CPU core temperatures."]
    })
}

fn capabilities() -> Value {
    json!({ "capabilities": [{ "name": "hardware.temperature", "description": "读取 CPU、GPU 和系统热区温度，并返回每个读数的数据来源。", "inputSchema": { "type": "object", "additionalProperties": false, "properties": {} } }] })
}

fn call(input: &[u8]) -> (i32, Value) {
    if input.len() as u64 > MAX_BUFFER_BYTES {
        return (2, json!({ "ok": false, "error": "input exceeds 1 MiB" }));
    }
    let Ok(request) = serde_json::from_slice::<Value>(input) else {
        return (3, json!({ "ok": false, "error": "invalid JSON" }));
    };
    let response = match request.get("method").and_then(Value::as_str) {
        Some("health") => json!({ "ok": true, "message": "hardware temperature ready" }),
        Some("capabilities") => capabilities(),
        Some("invoke")
            if request.get("capability").and_then(Value::as_str)
                == Some("hardware.temperature") =>
        {
            json!({ "ok": true, "value": collect_temperatures() })
        }
        Some("invoke") => json!({ "ok": false, "error": "unknown capability" }),
        _ => json!({ "ok": false, "error": "unknown method" }),
    };
    (0, response)
}

fn write_value(out: *mut Buffer, value: &Value) -> i32 {
    if out.is_null() {
        return 1;
    }
    let Ok(bytes) = serde_json::to_vec(value) else {
        return 4;
    };
    if bytes.is_empty() || bytes.len() as u64 > MAX_BUFFER_BYTES {
        return 4;
    }
    let boxed = bytes.into_boxed_slice();
    unsafe {
        (*out).len = boxed.len() as u64;
        (*out).data = Box::into_raw(boxed) as *mut u8;
    }
    0
}

#[no_mangle]
pub extern "C" fn pet_plugin_api_version() -> u32 {
    1
}
#[no_mangle]
pub unsafe extern "C" fn pet_plugin_init(_host: *const HostApi, out: *mut *mut c_void) -> i32 {
    if out.is_null() {
        return 1;
    }
    *out = Box::into_raw(Box::new(())) as *mut c_void;
    0
}
#[no_mangle]
pub unsafe extern "C" fn pet_plugin_call(
    _handle: *mut c_void,
    input: *const u8,
    len: u64,
    out: *mut Buffer,
) -> i32 {
    if input.is_null() || len > MAX_BUFFER_BYTES {
        return 2;
    }
    let (code, response) = call(std::slice::from_raw_parts(input, len as usize));
    if code != 0 {
        return code;
    }
    write_value(out, &response)
}
#[no_mangle]
pub unsafe extern "C" fn pet_plugin_shutdown(handle: *mut c_void) {
    if !handle.is_null() {
        drop(Box::from_raw(handle as *mut ()));
    }
}
#[no_mangle]
pub unsafe extern "C" fn pet_plugin_free_buffer(buffer: *mut Buffer) {
    if buffer.is_null() || (*buffer).data.is_null() {
        return;
    }
    let data = std::slice::from_raw_parts_mut((*buffer).data, (*buffer).len as usize);
    drop(Box::from_raw(data));
    (*buffer).data = std::ptr::null_mut();
    (*buffer).len = 0;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classifies_common_sensors() {
        assert_eq!(
            classify_sensor("CPU Package", "Intel CPU", "cpu/0"),
            Some("cpu")
        );
        assert_eq!(classify_sensor("GPU Core", "NVIDIA", "gpu/0"), Some("gpu"));
        assert_eq!(classify_sensor("Temperature", "Mainboard", "board/0"), None);
        assert_eq!(
            classify_hardware("GpuNvidia", "NVIDIA GeForce", "GPU Core"),
            Some("gpu")
        );
        assert_eq!(
            classify_hardware("CPU", "AMD Ryzen", "Core (Tctl/Tdie)"),
            Some("cpu")
        );
        assert_eq!(
            classify_hardware("Motherboard", "ASUS", "CPU Temperature"),
            Some("cpu")
        );
        assert_eq!(classify_hardware("Motherboard", "ASUS", "System"), None);
    }
    #[test]
    fn validates_celsius_values() {
        assert_eq!(value_to_celsius("42.345"), Some(42.35));
        assert_eq!(value_to_celsius("250"), None);
        assert_eq!(value_to_celsius("bad"), None);
    }
    #[test]
    fn exposes_temperature_capability() {
        assert_eq!(
            capabilities()["capabilities"][0]["name"],
            "hardware.temperature"
        );
    }
}
