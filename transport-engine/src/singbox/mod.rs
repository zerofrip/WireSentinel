pub mod config;
mod transport;

pub use config::{build_config, SingBoxOutboundSpec, SingBoxProtocol};
pub use transport::SingBoxTransport;
