use crate::socks5::Socks5Backend;
use shared_types::ProxyProfile;

pub struct HttpsProxyBackend {
    inner: Socks5Backend,
}

impl HttpsProxyBackend {
    pub fn new(profile: ProxyProfile) -> Self {
        Self {
            inner: Socks5Backend::new(profile),
        }
    }
}

#[async_trait::async_trait]
impl crate::backend::ProxyBackend for HttpsProxyBackend {
    fn profile_id(&self) -> uuid::Uuid {
        self.inner.profile_id()
    }

    fn profile(&self) -> &ProxyProfile {
        self.inner.profile()
    }

    async fn connect(&self) -> shared_types::Result<u16> {
        self.inner.connect().await
    }

    async fn disconnect(&self) -> shared_types::Result<()> {
        self.inner.disconnect().await
    }

    async fn health_check(&self) -> crate::backend::ProxyHealth {
        self.inner.health_check().await
    }

    async fn measure_latency(&self) -> shared_types::Result<u64> {
        self.inner.measure_latency().await
    }

    fn status(&self) -> crate::backend::ProxyStatus {
        self.inner.status()
    }
}
