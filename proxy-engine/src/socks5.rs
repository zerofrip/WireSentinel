use crate::backend::{ProxyBackend, ProxyHealth, ProxyState, ProxyStatus};
use async_trait::async_trait;
use parking_lot::RwLock;
use shared_types::{ProxyKind, ProxyProfile, Result, WireSentinelError};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tracing::{debug, warn};
use uuid::Uuid;

pub fn create_backend(profile: ProxyProfile) -> Arc<dyn ProxyBackend> {
    match profile.kind {
        ProxyKind::Socks5 => Arc::new(Socks5Backend::new(profile)),
        ProxyKind::Http => Arc::new(crate::http::HttpProxyBackend::new(profile)),
        ProxyKind::Https => Arc::new(crate::https::HttpsProxyBackend::new(profile)),
    }
}

pub struct Socks5Backend {
    profile: ProxyProfile,
    listen_port: RwLock<Option<u16>>,
    state: RwLock<ProxyState>,
    last_error: RwLock<Option<String>>,
    shutdown: RwLock<Option<watch::Sender<bool>>>,
    relay_task: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl Socks5Backend {
    pub fn new(profile: ProxyProfile) -> Self {
        Self {
            profile,
            listen_port: RwLock::new(None),
            state: RwLock::new(ProxyState::Disconnected),
            last_error: RwLock::new(None),
            shutdown: RwLock::new(None),
            relay_task: RwLock::new(None),
        }
    }

    async fn probe_upstream(&self) -> Result<()> {
        let addr = format!("{}:{}", self.profile.host, self.profile.port);
        let stream = tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&addr))
            .await
            .map_err(|_| WireSentinelError::Proxy(format!("upstream timeout: {addr}")))?
            .map_err(|e| WireSentinelError::Proxy(format!("upstream connect {addr}: {e}")))?;
        drop(stream);
        Ok(())
    }
}

#[async_trait]
impl ProxyBackend for Socks5Backend {
    fn profile_id(&self) -> Uuid {
        self.profile.id
    }

    fn profile(&self) -> &ProxyProfile {
        &self.profile
    }

    async fn connect(&self) -> Result<u16> {
        self.probe_upstream().await?;
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|e| WireSentinelError::Proxy(format!("local bind: {e}")))?;
        let local_port = listener
            .local_addr()
            .map_err(|e| WireSentinelError::Proxy(e.to_string()))?
            .port();

        let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
        *self.shutdown.write() = Some(shutdown_tx);

        let upstream_host = self.profile.host.clone();
        let upstream_port = self.profile.port;
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
                                    let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
                                }
                                Err(e) => warn!(%peer, err = %e, "proxy relay upstream failed"),
                            }
                        });
                    }
                }
            }
            debug!(port = local_port, "local socks relay stopped");
        });

        *self.relay_task.write() = Some(task);
        *self.listen_port.write() = Some(local_port);
        *self.state.write() = ProxyState::Connected;
        *self.last_error.write() = None;
        Ok(local_port)
    }

    async fn disconnect(&self) -> Result<()> {
        if let Some(tx) = self.shutdown.write().take() {
            let _ = tx.send(true);
        }
        if let Some(task) = self.relay_task.write().take() {
            task.abort();
        }
        *self.listen_port.write() = None;
        *self.state.write() = ProxyState::Disconnected;
        Ok(())
    }

    async fn health_check(&self) -> ProxyHealth {
        match self.probe_upstream().await {
            Ok(()) => ProxyHealth {
                healthy: true,
                message: None,
            },
            Err(e) => {
                let msg = e.to_string();
                *self.last_error.write() = Some(msg.clone());
                *self.state.write() = ProxyState::Failed;
                ProxyHealth {
                    healthy: false,
                    message: Some(msg),
                }
            }
        }
    }

    async fn measure_latency(&self) -> Result<u64> {
        let addr = format!("{}:{}", self.profile.host, self.profile.port);
        let start = std::time::Instant::now();
        tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(&addr))
            .await
            .map_err(|_| WireSentinelError::Proxy(format!("latency timeout: {addr}")))?
            .map_err(|e| WireSentinelError::Proxy(format!("latency connect {addr}: {e}")))?;
        Ok(start.elapsed().as_millis() as u64)
    }

    fn status(&self) -> ProxyStatus {
        ProxyStatus {
            state: *self.state.read(),
            listen_port: *self.listen_port.read(),
            last_error: self.last_error.read().clone(),
        }
    }
}
