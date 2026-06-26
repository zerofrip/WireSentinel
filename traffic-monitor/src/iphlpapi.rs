//! iphlpapi polling backend (full scan + diff).

use crate::backend::{BackendMode, ConnectionBackend, MonitorConnectionSink, MonitorContext};
use async_trait::async_trait;

pub struct IphlpapiBackend;

#[async_trait]
impl ConnectionBackend for IphlpapiBackend {
    fn mode(&self) -> BackendMode {
        BackendMode::Poll
    }

    fn name(&self) -> &'static str {
        "iphlpapi"
    }

    async fn run(&self, ctx: MonitorContext) -> Result<(), String> {
        let sink = MonitorConnectionSink::new(ctx.monitor.clone(), ctx.handler);
        let interval = ctx.monitor.poll_interval_ms();
        let mut ticker = tokio::time::interval(std::time::Duration::from_millis(interval));
        let mut shutdown = ctx.shutdown;
        let mut bootstrapped = false;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let connections = collect_tcp_connections();
                    sink.prune_to_active(&connections);
                    if !bootstrapped {
                        for conn in &connections {
                            sink.seed_known(conn);
                        }
                        bootstrapped = true;
                    } else {
                        for conn in connections {
                            sink.try_new_connection(conn);
                        }
                    }
                }
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}

fn collect_tcp_connections() -> Vec<shared_types::ConnectionSnapshot> {
    #[cfg(windows)]
    {
        crate::windows::enumerate_tcp_connections()
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}
