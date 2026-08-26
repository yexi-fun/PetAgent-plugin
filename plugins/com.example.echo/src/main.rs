use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

fn response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.len() > 1024 * 1024 { break; }
        let Ok(request) = serde_json::from_str::<Value>(&line) else { continue };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request.get("method").and_then(Value::as_str).unwrap_or_default();
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let output = match method {
            "initialize" => response(&id, json!({ "protocolVersion": "2026-01-01", "capabilities": { "tools": {} } })),
            "health" => response(&id, json!({ "ok": true, "message": "" })),
            "tools/list" | "capabilities" => response(&id, json!({ "capabilities": [{ "name": "echo", "description": "Echo text back to the caller.", "inputSchema": { "type": "object", "properties": { "text": { "type": "string" } }, "required": ["text"] } }] })),
            "tools/call" => {
                let text = params.get("arguments").and_then(|value| value.get("text")).and_then(Value::as_str).unwrap_or_default();
                response(&id, json!({ "content": [{ "type": "text", "text": text }], "isError": false }))
            }
            "shutdown" => response(&id, Value::Null),
            _ => error(&id, -32601, "method not found"),
        };
        if serde_json::to_writer(&mut stdout, &output).is_err() { break; }
        if stdout.write_all(b"\n").and_then(|_| stdout.flush()).is_err() { break; }
    }
}
