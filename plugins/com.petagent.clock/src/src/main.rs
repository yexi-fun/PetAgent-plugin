use chrono::Local;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

const TOOL_NAME: &str = "clock_now";
const MAX_LINE_BYTES: usize = 1024 * 1024;

fn response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn descriptor() -> Value {
    json!({
        "name": TOOL_NAME,
        "description": "获取当前本地时间，并在桌宠旁的独立气泡中显示。",
        "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false }
    })
}

fn now() -> Value {
    let formatted = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    json!({
        "now": formatted,
        "timezone": Local::now().offset().to_string(),
        "bubble": { "text": format!("现在时间：{formatted}") }
    })
}

fn handle(request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return error(&id, -32600, "invalid JSON-RPC version");
    }
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let params = request.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => response(
            &id,
            json!({ "protocolVersion": "2026-01-01", "capabilities": { "tools": {} }, "serverInfo": { "name": "petagent-clock-mcp", "version": env!("CARGO_PKG_VERSION") } }),
        ),
        "health" => response(&id, json!({ "ok": true, "message": "ready" })),
        "capabilities" | "tools/list" => response(
            &id,
            json!({ "capabilities": [descriptor()], "tools": [descriptor()] }),
        ),
        "tools/call" => {
            if params.get("name").and_then(Value::as_str) != Some(TOOL_NAME) {
                return error(&id, -32602, "unknown tool");
            }
            let text = serde_json::to_string(&now())
                .unwrap_or_else(|_| "{\"error\":\"serialize failed\"}".into());
            response(
                &id,
                json!({ "content": [{ "type": "text", "text": text }], "isError": false }),
            )
        }
        "shutdown" => response(&id, Value::Null),
        _ => error(&id, -32601, "method not found"),
    }
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.len() > MAX_LINE_BYTES {
            break;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let shutdown = request.get("method").and_then(Value::as_str) == Some("shutdown");
        if serde_json::to_writer(&mut stdout, &handle(&request)).is_err() {
            break;
        }
        if stdout
            .write_all(b"\n")
            .and_then(|_| stdout.flush())
            .is_err()
            || shutdown
        {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_clock_and_bubble_contract() {
        let value = descriptor();
        assert_eq!(value["name"], TOOL_NAME);
        assert!(value["description"].as_str().unwrap().contains("气泡"));
    }

    #[test]
    fn returns_local_time_payload() {
        let value = now();
        assert!(value["now"].as_str().is_some_and(|text| text.len() == 19));
        assert!(value["bubble"]["text"].as_str().is_some());
    }

    #[test]
    fn handles_tool_call() {
        let output = handle(
            &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": TOOL_NAME, "arguments": {} } }),
        );
        assert_eq!(output["result"]["isError"], false);
    }
}
