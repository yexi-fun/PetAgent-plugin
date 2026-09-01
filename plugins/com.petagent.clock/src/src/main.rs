use chrono::{Local, SecondsFormat};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_LINE_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
struct ClockState {
    hour12: bool,
    show_date: bool,
    visible: bool,
}

fn response(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn capabilities() -> Value {
    json!({ "capabilities": [
        {
            "name": "clock.now",
            "description": "返回系统本地时间。",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "events": ["clock.updated"]
        },
        {
            "name": "clock.show",
            "description": "显示独立时间窗口。",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "events": ["clock.visibility", "clock.updated"]
        },
        {
            "name": "clock.hide",
            "description": "隐藏独立时间窗口。",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
            "events": ["clock.visibility"]
        },
        {
            "name": "clock.set_format",
            "description": "设置 12/24 小时制和日期显示。",
            "inputSchema": {
                "type": "object",
                "properties": { "hour12": { "type": "boolean" }, "showDate": { "type": "boolean" } },
                "additionalProperties": false
            },
            "events": ["clock.updated"]
        }
    ]})
}

fn snapshot(state: &ClockState) -> Value {
    let now = Local::now();
    let time_format = if state.hour12 {
        "%I:%M:%S %p"
    } else {
        "%H:%M:%S"
    };
    json!({
        "text": now.format(time_format).to_string(),
        "date": state.show_date.then(|| now.format("%Y-%m-%d").to_string()),
        "iso": now.to_rfc3339_opts(SecondsFormat::Secs, true),
        "timezone": now.offset().to_string(),
        "source": "windows-system-clock"
    })
}

fn notification(name: &str, payload: Value) -> Value {
    json!({ "jsonrpc": "2.0", "method": "pet.app.event", "params": { "name": name, "payload": payload } })
}

fn write_message(stdout: &Arc<Mutex<io::Stdout>>, value: &Value) -> bool {
    let mut stdout = stdout
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    serde_json::to_writer(&mut *stdout, value).is_ok()
        && stdout.write_all(b"\n").and_then(|_| stdout.flush()).is_ok()
}

fn invoke(
    capability: &str,
    input: &Value,
    state: &Arc<Mutex<ClockState>>,
) -> Result<(Value, Vec<Value>), &'static str> {
    let mut state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    match capability {
        "clock.now" => {
            state.visible = true;
            let value = snapshot(&state);
            let mut result = value.clone();
            result["hostAction"] = json!({ "type": "window.open" });
            Ok((result, vec![notification("clock.updated", value)]))
        }
        "clock.show" => {
            state.visible = true;
            let value = snapshot(&state);
            Ok((
                json!({ "visible": true, "hostAction": { "type": "window.open" } }),
                vec![
                    notification("clock.visibility", json!({ "visible": true })),
                    notification("clock.updated", value),
                ],
            ))
        }
        "clock.hide" => {
            state.visible = false;
            Ok((
                json!({ "visible": false, "hostAction": { "type": "window.close" } }),
                vec![notification(
                    "clock.visibility",
                    json!({ "visible": false }),
                )],
            ))
        }
        "clock.set_format" => {
            if let Some(hour12) = input.get("hour12").and_then(Value::as_bool) {
                state.hour12 = hour12;
            }
            if let Some(show_date) = input.get("showDate").and_then(Value::as_bool) {
                state.show_date = show_date;
            }
            let value = snapshot(&state);
            Ok((
                json!({ "hour12": state.hour12, "showDate": state.show_date }),
                vec![notification("clock.updated", value)],
            ))
        }
        _ => Err("unknown capability"),
    }
}

fn main() {
    let state = Arc::new(Mutex::new(ClockState {
        hour12: false,
        show_date: true,
        visible: false,
    }));
    let stdout = Arc::new(Mutex::new(io::stdout()));
    let ticker_state = state.clone();
    let ticker_stdout = stdout.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(1));
        let state = ticker_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if state.visible
            && !write_message(
                &ticker_stdout,
                &notification("clock.updated", snapshot(&state)),
            )
        {
            break;
        }
    });

    for line in io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        if line.len() > MAX_LINE_BYTES {
            break;
        }
        let Ok(request) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = request.get("params").cloned().unwrap_or(Value::Null);
        let (reply, events, shutdown) = match method {
            "initialize" => (
                response(
                    &id,
                    json!({ "appProtocolVersion": 1, "serviceVersion": env!("CARGO_PKG_VERSION") }),
                ),
                Vec::new(),
                false,
            ),
            "health" => (
                response(&id, json!({ "ok": true, "message": "ready" })),
                Vec::new(),
                false,
            ),
            "capabilities" => (response(&id, capabilities()), Vec::new(), false),
            "invoke" => match invoke(
                params
                    .get("capability")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                params.get("input").unwrap_or(&Value::Null),
                &state,
            ) {
                Ok((result, events)) => (response(&id, result), events, false),
                Err(message) => (error(&id, -32602, message), Vec::new(), false),
            },
            "shutdown" => (response(&id, json!({ "ok": true })), Vec::new(), true),
            _ => (error(&id, -32601, "method not found"), Vec::new(), false),
        };
        if !write_message(&stdout, &reply) {
            break;
        }
        for event in events {
            if !write_message(&stdout, &event) {
                return;
            }
        }
        if shutdown {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_all_declared_capabilities() {
        let value = capabilities();
        let names = value["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["name"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            ["clock.now", "clock.show", "clock.hide", "clock.set_format"]
        );
    }

    #[test]
    fn uses_system_clock_and_updates_format() {
        let state = Arc::new(Mutex::new(ClockState {
            hour12: false,
            show_date: true,
            visible: false,
        }));
        let (_, events) = invoke(
            "clock.set_format",
            &json!({ "hour12": true, "showDate": false }),
            &state,
        )
        .unwrap();
        assert_eq!(events[0]["params"]["name"], "clock.updated");
        assert!(!state.lock().unwrap().show_date);
    }

    #[test]
    fn requests_window_actions_for_visibility_changes() {
        let state = Arc::new(Mutex::new(ClockState {
            hour12: false,
            show_date: true,
            visible: false,
        }));
        let (now, _) = invoke("clock.now", &json!({}), &state).unwrap();
        assert_eq!(now["hostAction"]["type"], "window.open");
        assert!(state.lock().unwrap().visible);
        let (hide, _) = invoke("clock.hide", &json!({}), &state).unwrap();
        assert_eq!(hide["hostAction"]["type"], "window.close");
    }
}
