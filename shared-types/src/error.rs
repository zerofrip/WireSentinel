use thiserror::Error;

#[derive(Debug, Error)]
pub enum WireSentinelError {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("policy error: {0}")]
    Policy(String),

    #[error("vpn error: {0}")]
    Vpn(String),

    #[error("wfp error: {0}")]
    Wfp(String),

    #[error("dns error: {0}")]
    Dns(String),

    #[error("traffic monitor error: {0}")]
    Traffic(String),

    #[error("api error: {0}")]
    Api(String),

    #[error("proxy error: {0}")]
    Proxy(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, WireSentinelError>;
