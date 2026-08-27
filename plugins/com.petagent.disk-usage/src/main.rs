use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use sysinfo::Disks;

const MAX_LINE_BYTES: usize = 1024 * 1024;
const TOOL_NAME: &str = "disk_usage";
const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

fn response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn round_two(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn usage_percent(used: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        round_two(used as f64 * 100.0 / total as f64)
    }
}

fn bytes_to_gib(bytes: u64) -> f64 {
    round_two(bytes as f64 / GIB)
}

fn disk_usage() -> Value {
    let disks = Disks::new_with_refreshed_list();
    let mut total_bytes = 0_u64;
    let mut available_bytes = 0_u64;
    let mut entries = Vec::with_capacity(disks.list().len());

    for disk in disks.list() {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        total_bytes = total_bytes.saturating_add(total);
        available_bytes = available_bytes.saturating_add(available);

        entries.push(json!({
            "mountPoint": disk.mount_point().to_string_lossy(),
            "name": disk.name().to_string_lossy(),
            "fileSystem": disk.file_system().to_string_lossy(),
            "kind": format!("{:?}", disk.kind()).to_ascii_lowercase(),
            "isRemovable": disk.is_removable(),
            "totalBytes": total,
            "usedBytes": used,
            "availableBytes": available,
            "totalGiB": bytes_to_gib(total),
            "usedGiB": bytes_to_gib(used),
            "availableGiB": bytes_to_gib(available),
            "usagePercent": usage_percent(used, total)
        }));
    }

    let used_bytes = total_bytes.saturating_sub(available_bytes);
    json!({
        "diskCount": entries.len(),
        "totalBytes": total_bytes,
        "usedBytes": used_bytes,
        "availableBytes": available_bytes,
        "totalGiB": bytes_to_gib(total_bytes),
        "usedGiB": bytes_to_gib(used_bytes),
        "availableGiB": bytes_to_gib(available_bytes),
        "usagePercent": usage_percent(used_bytes, total_bytes),
        "disks": entries
    })
}

fn tool_descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "获取系统各磁盘的总容量、已用空间、可用空间和占用百分比。",
        "inputSchema": {
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }
    })
}

fn handle_request(request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return error(&id, -32600, "invalid JSON-RPC version");
    }

    let Some(method) = request.get("method").and_then(Value::as_str) else {
        return error(&id, -32600, "method is required");
    };
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => response(
            &id,
            json!({
                "protocolVersion": "2026-01-01",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "disk-usage-mcp", "version": env!("CARGO_PKG_VERSION") }
            }),
        ),
        "health" => response(&id, json!({ "ok": true, "message": "ready" })),
        "capabilities" => response(&id, json!({ "capabilities": [tool_descriptor()] })),
        "tools/list" => response(&id, json!({ "tools": [tool_descriptor()] })),
        "tools/call" => {
            let name = params
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if name != TOOL_NAME {
                return error(&id, -32602, "unknown tool");
            }
            let data = disk_usage();
            let text = serde_json::to_string_pretty(&data)
                .unwrap_or_else(|_| "{\"error\":\"failed to serialize disk usage\"}".into());
            response(
                &id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "isError": false
                }),
            )
        }
        "shutdown" => response(&id, Value::Null),
        _ => error(&id, -32601, "method not found"),
    }
}

fn write_response(stdout: &mut impl Write, output: &Value) -> io::Result<()> {
    serde_json::to_writer(&mut *stdout, output)?;
    stdout.write_all(b"\n")?;
    stdout.flush()
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.strip_prefix('\u{feff}').unwrap_or(&line);
        let (output, shutdown) = if line.len() > MAX_LINE_BYTES {
            (error(&Value::Null, -32600, "request exceeds 1 MiB"), false)
        } else {
            match serde_json::from_str::<Value>(&line) {
                Ok(request) => {
                    let shutdown =
                        request.get("method").and_then(Value::as_str) == Some("shutdown");
                    (handle_request(&request), shutdown)
                }
                Err(_) => (error(&Value::Null, -32700, "parse error"), false),
            }
        };
        if write_response(&mut stdout, &output).is_err() || shutdown {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_usage_without_dividing_by_zero() {
        assert_eq!(usage_percent(25, 100), 25.0);
        assert_eq!(usage_percent(1, 3), 33.33);
        assert_eq!(usage_percent(0, 0), 0.0);
    }

    #[test]
    fn lists_disk_usage_tool() {
        let output = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/list",
            "params": {}
        }));

        assert_eq!(output["id"], 7);
        assert_eq!(output["result"]["tools"][0]["name"], TOOL_NAME);
        assert_eq!(
            output["result"]["tools"][0]["inputSchema"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn returns_current_disk_usage_as_text_content() {
        let output = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": "disk-test",
            "method": "tools/call",
            "params": { "name": TOOL_NAME, "arguments": {} }
        }));

        assert_eq!(output["result"]["isError"], false);
        let text = output["result"]["content"][0]["text"].as_str().unwrap();
        let payload: Value = serde_json::from_str(text).unwrap();
        assert!(payload["disks"].is_array());
        assert!(payload["diskCount"].is_number());
    }

    #[test]
    fn rejects_unknown_tool_names() {
        let output = handle_request(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": { "name": "missing", "arguments": {} }
        }));

        assert_eq!(output["error"]["code"], -32602);
    }
}
