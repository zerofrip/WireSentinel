//! WinDivert dynamic-load engine — signed-stack alternative to NDIS LWF redirect/transform.

mod capture;
mod packet_parse;
mod redirect;
mod telemetry;
mod transform;

#[cfg(windows)]
mod ffi;

#[cfg(windows)]
pub use capture::{capture_available, CapturedPacket, WinDivertCapture};
pub use packet_parse::{parse_packet, FlowEndpoints};

#[cfg(windows)]
pub use redirect::{windivert_available, WinDivertEngine, WinDivertEngineApi, WinDivertHealth};

#[cfg(not(windows))]
pub use redirect::{windivert_available, WinDivertEngine, WinDivertEngineApi, WinDivertHealth};

pub use telemetry::WinDivertTelemetry;
pub use transform::PacketTransformHook;
