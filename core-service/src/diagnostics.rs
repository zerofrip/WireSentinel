//! Diagnostics health checks and bundle export.

use chrono::Utc;
use dns::DnsLayer;
use shared_types::{DiagnosticsHealth, Result, SubsystemHealth, WireSentinelError};
use std::sync::Arc;
use storage::{data_dir, Storage};
use vpn_engine::VpnManager;
use wfp::WfpEngine;

use crate::transport::TransportManager;

pub struct DiagnosticsService {
    storage: Arc<Storage>,
    #[allow(dead_code)]
    wfp: Arc<dyn WfpEngine>,
    vpn: Arc<VpnManager>,
    dns: Arc<DnsLayer>,
    transport: Arc<TransportManager>,
}

impl DiagnosticsService {
    pub fn new(
        storage: Arc<Storage>,
        wfp: Arc<dyn WfpEngine>,
        vpn: Arc<VpnManager>,
        dns: Arc<DnsLayer>,
        transport: Arc<TransportManager>,
    ) -> Self {
        Self {
            storage,
            wfp,
            vpn,
            dns,
            transport,
        }
    }

    pub async fn health(&self) -> DiagnosticsHealth {
        DiagnosticsHealth {
            wfp: self.check_wfp().await,
            vpn: self.check_vpn().await,
            dns: self.check_dns(),
            transport: self.check_transport().await,
            database: self.check_database().await,
            disk: self.check_disk(),
        }
    }

    async fn check_wfp(&self) -> SubsystemHealth {
        let driver = self.wfp.driver_state().await;
        let status = match driver.state.as_str() {
            "ok" | "running" | "loaded" => "ok",
            "stub" | "disabled" => "disabled",
            _ => "error",
        };
        SubsystemHealth {
            status: status.into(),
            message: driver
                .message
                .or(Some(format!("engine: {}", driver.engine))),
        }
    }

    async fn check_vpn(&self) -> SubsystemHealth {
        let count = self.vpn.active_count();
        SubsystemHealth {
            status: "ok".into(),
            message: Some(format!("{count} active tunnel(s)")),
        }
    }

    fn check_dns(&self) -> SubsystemHealth {
        let settings = self.dns.settings();
        SubsystemHealth {
            status: if settings.enabled { "ok" } else { "disabled" }.into(),
            message: None,
        }
    }

    async fn check_transport(&self) -> SubsystemHealth {
        match self.transport.status().await {
            Ok(rows) => SubsystemHealth {
                status: "ok".into(),
                message: Some(format!("{} profile(s)", rows.len())),
            },
            Err(e) => SubsystemHealth {
                status: "error".into(),
                message: Some(e.to_string()),
            },
        }
    }

    async fn check_database(&self) -> SubsystemHealth {
        match self.storage.settings.get_api_port().await {
            Ok(_) => SubsystemHealth {
                status: "ok".into(),
                message: None,
            },
            Err(e) => SubsystemHealth {
                status: "error".into(),
                message: Some(e.to_string()),
            },
        }
    }

    fn check_disk(&self) -> SubsystemHealth {
        let dir = data_dir();
        match std::fs::create_dir_all(&dir) {
            Ok(()) => SubsystemHealth {
                status: "ok".into(),
                message: Some(dir.display().to_string()),
            },
            Err(e) => SubsystemHealth {
                status: "error".into(),
                message: Some(e.to_string()),
            },
        }
    }

    pub async fn export_bundle(&self) -> Result<Vec<u8>> {
        use std::io::Write;

        let health = self.health().await;
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);

            let health_json =
                serde_json::to_string_pretty(&health).map_err(WireSentinelError::Serde)?;
            zip.start_file("health.json", options)
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;
            zip.write_all(health_json.as_bytes())
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;

            let version = serde_json::json!({
                "generated_at": Utc::now(),
                "version": env!("CARGO_PKG_VERSION"),
            });
            zip.start_file("version.json", options)
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;
            zip.write_all(
                serde_json::to_string_pretty(&version)
                    .map_err(WireSentinelError::Serde)?
                    .as_bytes(),
            )
            .map_err(|e| WireSentinelError::Config(e.to_string()))?;

            if let Ok(Some(s)) = self.storage.privacy_snapshots.latest().await {
                zip.start_file("privacy_snapshot.json", options)
                    .map_err(|e| WireSentinelError::Config(e.to_string()))?;
                zip.write_all(
                    serde_json::to_string_pretty(&s)
                        .map_err(WireSentinelError::Serde)?
                        .as_bytes(),
                )
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;
            }

            zip.finish()
                .map_err(|e| WireSentinelError::Config(e.to_string()))?;
        }
        Ok(buffer.into_inner())
    }
}
