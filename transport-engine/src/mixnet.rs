use crate::backend::{TransportBackend, TransportContext};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{Result, TransportHealth, TransportKind, TransportState, WireSentinelError};
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tracing::{info, warn};

/// Relays the chain listen port to a mixnet SOCKS endpoint started by `MixnetService`.
pub struct MixnetTransport {
    state: RwLock<TransportState>,
    shutdown: RwLock<Option<watch::Sender<bool>>>,
    relay_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl MixnetTransport {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(TransportState::Stopped),
            shutdown: RwLock::new(None),
            relay_task: RwLock::new(None),
        }
    }

    async fn probe_upstream(addr: &str) -> Result<()> {
        tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(addr))
            .await
            .map_err(|_| WireSentinelError::Other(format!("mixnet upstream timeout: {addr}")))?
            .map_err(|e| {
                WireSentinelError::Other(format!("mixnet upstream connect {addr}: {e}"))
            })?;
        Ok(())
    }
}

impl Default for MixnetTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TransportBackend for MixnetTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::Mixnet
    }

    async fn start(&self, ctx: &TransportContext) -> Result<()> {
        let upstream = ctx.mixnet_upstream.as_ref().ok_or_else(|| {
            WireSentinelError::Other(
                "mixnet hop requires mixnet_upstream; start mixnet profile via MixnetService first"
                    .into(),
            )
        })?;
        let listen_port = ctx.listen_port.ok_or_else(|| {
            WireSentinelError::Other("mixnet hop requires listen_port in transport context".into())
        })?;

        Self::probe_upstream(upstream).await?;

        let listener = TcpListener::bind(format!("127.0.0.1:{listen_port}"))
            .await
            .map_err(|e| WireSentinelError::Other(format!("mixnet local bind: {e}")))?;

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        *self.shutdown.write() = Some(shutdown_tx);

        let upstream_addr = upstream.clone();
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
                        let addr = upstream_addr.clone();
                        tokio::spawn(async move {
                            match TcpStream::connect(addr.as_str()).await {
                                Ok(mut upstream) => {
                                    let _ = copy_bidirectional(&mut client, &mut upstream).await;
                                }
                                Err(e) => warn!(%peer, err = %e, "mixnet relay upstream failed"),
                            }
                        });
                    }
                }
            }
            info!(name = %name, port = listen_port, "mixnet transport relay stopped");
        });

        *self.relay_task.write() = Some(task);
        *self.state.write() = TransportState::Running;
        info!(
            name = %ctx.name,
            port = listen_port,
            upstream = %upstream,
            "mixnet transport relay started"
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
                Some("mixnet relay active".into())
            } else {
                Some("mixnet relay stopped; use POST /mixnet/start".into())
            },
        }
    }
}
