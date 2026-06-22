//! Windows native validation framework.

use chrono::Utc;
use event_bus::EventBus;
use shared_types::{
    GuardianMode, Result, ServiceEventInner, ValidationCheck, ValidationReport, ValidationStatus,
};
use std::path::PathBuf;
use std::sync::Arc;
use storage::Storage;
use tracing::info;
use uuid::Uuid;

pub struct ValidationService {
    storage: Arc<Storage>,
    events: EventBus,
}

impl ValidationService {
    pub fn new(storage: Arc<Storage>, events: EventBus) -> Self {
        Self { storage, events }
    }

    pub async fn run_all(&self) -> Result<ValidationReport> {
        let checks = vec![
            self.check_os_version().await,
            self.check_wireguard_install().await,
            self.check_tunnel_dll().await,
            self.check_wireguard_dll().await,
            self.check_amnezia_runtime().await,
            self.check_wfp_availability().await,
            self.check_etw_availability().await,
            self.check_service_permissions().await,
            self.check_kernel_wfp_setting().await,
            self.check_guardian_mode().await,
            self.check_ndis_driver_setting().await,
            self.check_hybrid_health().await,
        ];

        for check in &checks {
            self.storage.validation_results.upsert(check).await?;
            match check.status {
                ValidationStatus::Pass => {
                    self.events.publish(
                        ServiceEventInner::ValidationPassed {
                            check_name: check.check_name.clone(),
                            message: check.message.clone(),
                        }
                        .with_timestamp(check.checked_at),
                    );
                }
                ValidationStatus::Fail => {
                    self.events.publish(
                        ServiceEventInner::ValidationFailed {
                            check_name: check.check_name.clone(),
                            message: check.message.clone().unwrap_or_default(),
                        }
                        .with_timestamp(check.checked_at),
                    );
                }
                ValidationStatus::Warn => {}
            }
        }

        let overall_status = if checks.iter().any(|c| c.status == ValidationStatus::Fail) {
            ValidationStatus::Fail
        } else if checks.iter().any(|c| c.status == ValidationStatus::Warn) {
            ValidationStatus::Warn
        } else {
            ValidationStatus::Pass
        };

        info!(
            ?overall_status,
            count = checks.len(),
            "validation completed"
        );

        Ok(ValidationReport {
            overall_status,
            checks,
            generated_at: Utc::now(),
        })
    }

    pub async fn latest_report(&self) -> Result<ValidationReport> {
        let checks = self.storage.validation_results.list_recent(20).await?;
        if checks.is_empty() {
            return self.run_all().await;
        }
        let overall_status = if checks.iter().any(|c| c.status == ValidationStatus::Fail) {
            ValidationStatus::Fail
        } else if checks.iter().any(|c| c.status == ValidationStatus::Warn) {
            ValidationStatus::Warn
        } else {
            ValidationStatus::Pass
        };
        Ok(ValidationReport {
            overall_status,
            checks,
            generated_at: Utc::now(),
        })
    }

    async fn check_os_version(&self) -> ValidationCheck {
        self.make_check("os_version", || {
            #[cfg(windows)]
            {
                use windows::Win32::System::SystemInformation::{GetVersionExW, OSVERSIONINFOW};
                let mut info = OSVERSIONINFOW {
                    dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
                    ..Default::default()
                };
                unsafe {
                    if GetVersionExW(&mut info).is_ok() {
                        let build = info.dwBuildNumber;
                        if build >= 18362 {
                            return Ok((ValidationStatus::Pass, Some(format!("build {build}"))));
                        }
                        return Ok((
                            ValidationStatus::Fail,
                            Some(format!("unsupported build {build}, need 18362+")),
                        ));
                    }
                }
                Ok((
                    ValidationStatus::Warn,
                    Some("could not read OS version".into()),
                ))
            }
            #[cfg(not(windows))]
            {
                Ok((
                    ValidationStatus::Warn,
                    Some("OS validation skipped on non-Windows".into()),
                ))
            }
        })
        .await
    }

    async fn check_wireguard_install(&self) -> ValidationCheck {
        self.make_check("wireguard_install", || {
            let paths = [
                PathBuf::from(r"C:\Program Files\WireGuard\wireguard.exe"),
                PathBuf::from(r"C:\Program Files\WireGuard\wg.exe"),
            ];
            for p in paths {
                if p.exists() {
                    return Ok((ValidationStatus::Pass, Some(p.display().to_string())));
                }
            }
            Ok((
                ValidationStatus::Warn,
                Some("WireGuard for Windows not found in default path".into()),
            ))
        })
        .await
    }

    async fn check_tunnel_dll(&self) -> ValidationCheck {
        self.check_dll("tunnel_dll", "tunnel.dll").await
    }

    async fn check_wireguard_dll(&self) -> ValidationCheck {
        self.check_dll("wireguard_dll", "wireguard.dll").await
    }

    async fn check_dll(&self, name: &str, file: &str) -> ValidationCheck {
        self.make_check(name, || {
            let exe = std::env::current_exe().unwrap_or_default();
            let dir = exe.parent().unwrap_or(std::path::Path::new("."));
            let path = dir.join(file);
            if path.exists() {
                Ok((ValidationStatus::Pass, Some(path.display().to_string())))
            } else {
                Ok((
                    ValidationStatus::Warn,
                    Some(format!("{file} not found next to service binary")),
                ))
            }
        })
        .await
    }

    async fn check_amnezia_runtime(&self) -> ValidationCheck {
        let impl_name = self
            .storage
            .settings
            .vpn_amnezia_impl()
            .await
            .unwrap_or_else(|_| "scm".into());
        ValidationCheck {
            id: Uuid::new_v4(),
            check_name: "amnezia_runtime".into(),
            status: ValidationStatus::Pass,
            message: Some(format!("backend: {impl_name}")),
            checked_at: Utc::now(),
        }
    }

    async fn check_wfp_availability(&self) -> ValidationCheck {
        self.make_check("wfp_availability", || {
            #[cfg(windows)]
            {
                use windows::core::PCWSTR;
                use windows::Win32::NetworkManagement::WindowsFilteringPlatform::{
                    FwpmEngineClose0, FwpmEngineOpen0, FWPM_SESSION0,
                };
                let mut handle = std::ptr::null_mut();
                let session = FWPM_SESSION0::default();
                unsafe {
                    FwpmEngineOpen0(
                        PCWSTR::null(),
                        0x00000002,
                        None,
                        Some(&session as *const _ as *mut _),
                        &mut handle,
                    )
                    .map_err(|e| WireSentinelError::Wfp(format!("{e}")))?;
                    FwpmEngineClose0(handle);
                }
                return Ok((ValidationStatus::Pass, Some("FwpmEngineOpen0 ok".into())));
            }
            #[cfg(not(windows))]
            {
                Ok((
                    ValidationStatus::Warn,
                    Some("WFP not available on this platform".into()),
                ))
            }
        })
        .await
    }

    async fn check_etw_availability(&self) -> ValidationCheck {
        self.make_check("etw_availability", || {
            #[cfg(windows)]
            {
                Ok((
                    ValidationStatus::Pass,
                    Some("tracing subscriber active".into()),
                ))
            }
            #[cfg(not(windows))]
            {
                Ok((ValidationStatus::Warn, Some("ETW skipped".into())))
            }
        })
        .await
    }

    async fn check_service_permissions(&self) -> ValidationCheck {
        self.make_check("service_permissions", || {
            #[cfg(windows)]
            {
                use windows::Win32::Security::Authorization::IsUserAnAdmin;
                unsafe {
                    if IsUserAnAdmin().as_bool() {
                        return Ok((ValidationStatus::Pass, Some("elevated".into())));
                    }
                }
                Ok((
                    ValidationStatus::Warn,
                    Some("not running elevated — WFP may fail".into()),
                ))
            }
            #[cfg(not(windows))]
            {
                Ok((ValidationStatus::Pass, Some("non-Windows dev mode".into())))
            }
        })
        .await
    }

    async fn check_kernel_wfp_setting(&self) -> ValidationCheck {
        let impl_name = self
            .storage
            .settings
            .wfp_engine_impl()
            .await
            .unwrap_or_else(|_| "userspace".into());
        let (status, message) = if impl_name == "kernel" {
            match wfp::kernel_driver_available() {
                Ok(info) => (ValidationStatus::Pass, Some(info)),
                Err(e) => (ValidationStatus::Fail, Some(e)),
            }
        } else {
            (ValidationStatus::Pass, Some(impl_name))
        };
        ValidationCheck {
            id: Uuid::new_v4(),
            check_name: "kernel_wfp_setting".into(),
            status,
            message,
            checked_at: Utc::now(),
        }
    }

    async fn check_guardian_mode(&self) -> ValidationCheck {
        let mode = self
            .storage
            .settings
            .guardian_mode()
            .await
            .unwrap_or_else(|_| "wfp".into());
        let parsed = GuardianMode::parse(&mode);
        ValidationCheck {
            id: Uuid::new_v4(),
            check_name: "guardian_mode".into(),
            status: ValidationStatus::Pass,
            message: Some(format!("mode={}", parsed.as_str())),
            checked_at: Utc::now(),
        }
    }

    async fn check_ndis_driver_setting(&self) -> ValidationCheck {
        let mode = GuardianMode::parse(
            &self
                .storage
                .settings
                .guardian_mode()
                .await
                .unwrap_or_else(|_| "wfp".into()),
        );
        if !mode.uses_ndis() {
            return ValidationCheck {
                id: Uuid::new_v4(),
                check_name: "ndis_driver".into(),
                status: ValidationStatus::Pass,
                message: Some("not required for wfp mode".into()),
                checked_at: Utc::now(),
            };
        }
        let (status, message) = match wfp::ndis_driver_available() {
            Ok(info) => (ValidationStatus::Pass, Some(info)),
            Err(e) if mode == GuardianMode::Ndis => (ValidationStatus::Fail, Some(e)),
            Err(e) => (ValidationStatus::Warn, Some(e)),
        };
        ValidationCheck {
            id: Uuid::new_v4(),
            check_name: "ndis_driver".into(),
            status,
            message,
            checked_at: Utc::now(),
        }
    }

    async fn check_hybrid_health(&self) -> ValidationCheck {
        let mode = GuardianMode::parse(
            &self
                .storage
                .settings
                .guardian_mode()
                .await
                .unwrap_or_else(|_| "wfp".into()),
        );
        if mode != GuardianMode::Hybrid {
            return ValidationCheck {
                id: Uuid::new_v4(),
                check_name: "hybrid_health".into(),
                status: ValidationStatus::Pass,
                message: Some("not in hybrid mode".into()),
                checked_at: Utc::now(),
            };
        }
        let guardian = wfp::kernel_driver_available();
        let ndis = wfp::ndis_driver_available();
        let (status, message) = match (guardian, ndis) {
            (Ok(g), Ok(n)) => (
                ValidationStatus::Pass,
                Some(format!("guardian={g}; ndis={n}")),
            ),
            (Err(g), Ok(n)) => (
                ValidationStatus::Fail,
                Some(format!("guardian unavailable: {g}; ndis={n}")),
            ),
            (Ok(g), Err(n)) => (
                ValidationStatus::Warn,
                Some(format!("guardian={g}; ndis unavailable: {n}")),
            ),
            (Err(g), Err(n)) => (
                ValidationStatus::Fail,
                Some(format!("guardian: {g}; ndis: {n}")),
            ),
        };
        ValidationCheck {
            id: Uuid::new_v4(),
            check_name: "hybrid_health".into(),
            status,
            message,
            checked_at: Utc::now(),
        }
    }

    async fn make_check<F>(&self, name: &str, f: F) -> ValidationCheck
    where
        F: FnOnce() -> Result<(ValidationStatus, Option<String>)>,
    {
        let now = Utc::now();
        match f() {
            Ok((status, message)) => ValidationCheck {
                id: Uuid::new_v4(),
                check_name: name.into(),
                status,
                message,
                checked_at: now,
            },
            Err(e) => ValidationCheck {
                id: Uuid::new_v4(),
                check_name: name.into(),
                status: ValidationStatus::Fail,
                message: Some(e.to_string()),
                checked_at: now,
            },
        }
    }
}
