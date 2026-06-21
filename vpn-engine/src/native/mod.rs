//! Native WireGuard / AmneziaWG backends via dynamically loaded DLLs (Windows).

#[cfg(windows)]
mod imp;

#[cfg(not(windows))]
mod stub;

mod amnezia;

#[cfg(windows)]
pub use imp::NativeWireGuardBackend;

#[cfg(not(windows))]
pub use stub::NativeWireGuardBackend;

pub use amnezia::NativeAmneziaWgBackend;
