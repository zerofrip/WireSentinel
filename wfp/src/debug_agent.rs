//! Agent debug logging for WFP diagnostics (session 28de1e).

#[cfg(windows)]
pub fn agent_log(hypothesis_id: &str, location: &str, message: &str, data: &str) {
    use std::io::Write;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let line = format!(
        r#"{{"sessionId":"28de1e","hypothesisId":"{hypothesis_id}","location":"{location}","message":"{message}","data":{data},"timestamp":{ts}}}"#
    );
    // #region agent log
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug-28de1e.log")
    {
        let _ = writeln!(file, "{line}");
    }
    // #endregion
}
