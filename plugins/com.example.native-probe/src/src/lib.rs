use std::ffi::c_void;

#[repr(C)] pub struct HostApi { pub struct_size: u32, pub api_version: u32 }
#[repr(C)] pub struct Buffer { pub data: *mut u8, pub len: u64 }

#[no_mangle]
pub extern "C" fn pet_plugin_api_version() -> u32 { 1 }

#[no_mangle]
pub unsafe extern "C" fn pet_plugin_init(_host: *const HostApi, out: *mut *mut c_void) -> i32 {
    if out.is_null() { return 1; }
    *out = Box::into_raw(Box::new(())) as *mut c_void;
    0
}

#[no_mangle]
pub unsafe extern "C" fn pet_plugin_call(_handle: *mut c_void, input: *const u8, len: u64, out: *mut Buffer) -> i32 {
    if input.is_null() || out.is_null() || len > 1024 * 1024 { return 2; }
    let request = std::slice::from_raw_parts(input, len as usize);
    let value: serde_json::Value = serde_json::from_slice(request).unwrap_or_default();
    let response = match value.get("method").and_then(serde_json::Value::as_str) {
        Some("health") => serde_json::json!({"ok":true,"message":"native probe ready"}),
        Some("capabilities") => serde_json::json!({"capabilities":[{"name":"native.probe","description":"Return a native ABI probe result.","inputSchema":{"type":"object"}}]}),
        Some("invoke") => serde_json::json!({"ok":true,"value":"native probe"}),
        _ => serde_json::json!({"ok":false,"error":"unknown method"}),
    };
    let bytes = serde_json::to_vec(&response).unwrap_or_default();
    let boxed = bytes.into_boxed_slice();
    (*out).len = boxed.len() as u64;
    (*out).data = Box::into_raw(boxed) as *mut u8;
    0
}

#[no_mangle]
pub unsafe extern "C" fn pet_plugin_shutdown(handle: *mut c_void) { if !handle.is_null() { drop(Box::from_raw(handle as *mut ())); } }

#[no_mangle]
pub unsafe extern "C" fn pet_plugin_free_buffer(buffer: *mut Buffer) {
    if !buffer.is_null() && !(*buffer).data.is_null() { let _ = Box::from_raw(std::slice::from_raw_parts_mut((*buffer).data, (*buffer).len as usize)); (*buffer).data = std::ptr::null_mut(); (*buffer).len = 0; }
}
