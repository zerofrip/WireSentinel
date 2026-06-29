//! TEMPORARY debug-session instrumentation (session 28de1e).
//!
//! Appends NDJSON lines to a file the Windows service can write. The default
//! path is under the data dir (C:\ProgramData\WireSentinel on Windows), which is
//! readable from WSL at /mnt/c/ProgramData/WireSentinel. Override with the
//! `WS_DEBUG_LOG` environment variable. This module is expected to be removed
//! once the CPU/memory issue is verified fixed.

use std::io::Write;

const SESSION_ID: &str = "28de1e";

fn log_path() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("WS_DEBUG_LOG") {
        return std::path::PathBuf::from(p);
    }
    if cfg!(windows) {
        let base = std::env::var("PROGRAMDATA").unwrap_or_else(|_| r"C:\ProgramData".to_string());
        std::path::PathBuf::from(base)
            .join("WireSentinel")
            .join(format!("debug-{SESSION_ID}.log"))
    } else {
        std::path::PathBuf::from(format!("/tmp/WireSentinel/debug-{SESSION_ID}.log"))
    }
}

fn write_line(line: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "{line}");
    }
}

/// Append one NDJSON event. Best-effort: never panics, never blocks the caller
/// on errors (a failed write is silently ignored).
pub fn emit(location: &str, message: &str, data: serde_json::Value) {
    let line = serde_json::json!({
        "sessionId": SESSION_ID,
        "timestamp": chrono::Utc::now().timestamp_millis(),
        "location": location,
        "message": message,
        "data": data,
    });
    write_line(&line.to_string());
}

/// Dependency-free variant: callers pass string key/value pairs, so crates that
/// do not depend on `serde_json` can still emit structured events. Values are
/// embedded as raw JSON when they parse as JSON, otherwise as JSON strings.
pub fn emit_kv(location: &str, message: &str, kvs: &[(&str, String)]) {
    let mut data = String::from("{");
    for (i, (k, v)) in kvs.iter().enumerate() {
        if i > 0 {
            data.push(',');
        }
        let value = if serde_json::from_str::<serde_json::Value>(v).is_ok() {
            v.clone()
        } else {
            serde_json::Value::String(v.clone()).to_string()
        };
        data.push_str(&format!(
            "{}:{}",
            serde_json::Value::String(k.to_string()),
            value
        ));
    }
    data.push('}');
    let line = format!(
        "{{\"sessionId\":\"{SESSION_ID}\",\"timestamp\":{},\"location\":{},\"message\":{},\"data\":{}}}",
        chrono::Utc::now().timestamp_millis(),
        serde_json::Value::String(location.to_string()),
        serde_json::Value::String(message.to_string()),
        data,
    );
    write_line(&line);
}
