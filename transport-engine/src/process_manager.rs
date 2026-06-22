use parking_lot::RwLock;
use shared_types::Result;
#[cfg(windows)]
use shared_types::WireSentinelError;
use std::collections::HashMap;
use std::path::Path;
#[cfg(windows)]
use std::process::Stdio;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ManagedProcess {
    pid: u32,
    binary: String,
    config_path: String,
}

/// Spawns and supervises external transport binaries (sing-box, xray-core).
pub struct ProcessManager {
    processes: RwLock<HashMap<Uuid, ManagedProcess>>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self {
            processes: RwLock::new(HashMap::new()),
        }
    }

    pub fn is_running(&self, id: Uuid) -> bool {
        self.processes.read().contains_key(&id)
    }

    pub fn pid(&self, id: Uuid) -> Option<u32> {
        self.processes.read().get(&id).map(|p| p.pid)
    }

    pub async fn spawn(
        &self,
        id: Uuid,
        binary: &Path,
        args: &[&str],
        config_path: &Path,
    ) -> Result<()> {
        if self.is_running(id) {
            return Ok(());
        }

        #[cfg(windows)]
        {
            use tokio::process::Command;

            let mut cmd = Command::new(binary);
            cmd.args(args)
                .arg(config_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true);

            let child = cmd.spawn().map_err(|e| {
                WireSentinelError::Other(format!("spawn {}: {e}", binary.display()))
            })?;
            let pid = child.id().unwrap_or(0);

            self.processes.write().insert(
                id,
                ManagedProcess {
                    pid,
                    binary: binary.to_string_lossy().into_owned(),
                    config_path: config_path.to_string_lossy().into_owned(),
                },
            );
            info!(%id, pid, binary = %binary.display(), "transport process spawned");
            Ok(())
        }

        #[cfg(not(windows))]
        {
            let _ = (binary, args, config_path);
            self.processes.write().insert(
                id,
                ManagedProcess {
                    pid: 0,
                    binary: "stub".into(),
                    config_path: config_path.to_string_lossy().into_owned(),
                },
            );
            info!(%id, "stub transport process marked running (non-Windows)");
            Ok(())
        }
    }

    pub async fn kill(&self, id: Uuid) -> Result<()> {
        let entry = self.processes.write().remove(&id);
        let Some(_entry) = entry else {
            return Ok(());
        };

        #[cfg(windows)]
        {
            if entry.pid > 0 {
                use std::process::Command;
                let _ = Command::new("taskkill")
                    .args(["/PID", &entry.pid.to_string(), "/F", "/T"])
                    .status();
            }
            info!(%id, pid = entry.pid, "transport process killed");
        }

        #[cfg(not(windows))]
        {
            info!(%id, "stub transport process stopped");
        }

        Ok(())
    }

    pub async fn restart(
        &self,
        id: Uuid,
        binary: &Path,
        args: &[&str],
        config_path: &Path,
    ) -> Result<()> {
        if let Err(e) = self.kill(id).await {
            warn!(%id, error = %e, "kill before restart failed");
        }
        self.spawn(id, binary, args, config_path).await
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new()
    }
}
