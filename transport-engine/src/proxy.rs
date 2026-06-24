use crate::backend::{TransportBackend, TransportContext};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState, WireSentinelError};
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tracing::{info, warn};

/// Local TCP relay on the chain listen port → configured remote proxy.
pub struct ProxyTransport {
    state: RwLock<TransportState>,
    shutdown: RwLock<Option<watch::Sender<bool>>>,
    relay_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl ProxyTransport {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(TransportState::Stopped),
            shutdown: RwLock::new(None),
            relay_task: RwLock::new(None),
        }
    }

    async fn probe_upstream(host: &str, port: u16) -> Result<()> {
        let addr = format!("{host}:{port}");
        tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&addr))
            .await
            .map_err(|_| WireSentinelError::Proxy(format!("upstream timeout: {addr}")))?
            .map_err(|e| WireSentinelError::Proxy(format!("upstream connect {addr}: {e}")))?;
        Ok(())
    }
}

impl Default for ProxyTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportBackend for ProxyTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Proxy
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        let profile = ctx.proxy_profile.as_ref().ok_or_else(|| {
            WireSentinelError::Other("proxy hop requires proxy_profile in transport context".into())
        })?;
        let listen_port = ctx.listen_port.ok_or_else(|| {
            WireSentinelError::Other("proxy hop requires listen_port in transport context".into())
        })?;

        Self::probe_upstream(&profile.host, profile.port).await?;

        let listener = TcpListener::bind(format!("127.0.0.1:{listen_port}"))
            .await
            .map_err(|e| WireSentinelError::Proxy(format!("local bind: {e}")))?;

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        *self.shutdown.write() = Some(shutdown_tx);

        let upstream_host = profile.host.clone();
        let upstream_port = profile.port;
        let name = ctx.name.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    accept = listener.accept() => {
                        let Ok((mut client, peer)) = accept else { continue };
                        let host = upstream_host.clone();
                        tokio::spawn(async move {
                            match TcpStream::connect((host.as_str(), upstream_port)).await {
                                Ok(mut upstream) => {
                                    let _ = copy_bidirectional(&mut client, &mut upstream).await;
                                }
                                Err(e) => warn!(%peer, err = %e, "proxy relay upstream failed"),
                            }
                        });
                    }
                }
            }
            info!(name = %name, port = listen_port, "proxy transport relay stopped");
        });

        *self.relay_task.write() = Some(task);
        *self.state.write() = TransportState::Running;
        info!(
            name = %ctx.name,
            port = listen_port,
            upstream = %format!("{}:{}", profile.host, profile.port),
            "proxy transport started"
        );
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        if let Some(tx) = self.shutdown.write().take() {
            let _ = tx.send(true);
        }
        if let Some(task) = self.relay_task.write().take() {
            task.abort();
        }
        *self.state.write() = TransportState::Stopped;
        Ok(())
    }

    fn status(&self) -> TransportState {
        *self.state.read()
    }

    async fn health_check(&self) -> TransportHealth {
        let running = self.status() == TransportState::Running;
        TransportHealth {
            healthy: running,
            latency_ms: None,
            message: if running {
                Some("proxy relay active".into())
            } else {
                Some("proxy relay stopped".into())
            },
        }
    }
}
