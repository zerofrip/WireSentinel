//! Resolve external binary paths (sing-box, tor) next to the service executable.

use std::path::{Path, PathBuf};
use uuid::Uuid;

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resources_dir() -> PathBuf {
    let resources = exe_dir().join("resources");
    if resources.is_dir() {
        return resources;
    }
    exe_dir()
}

/// First existing path among override, env, `resources/`, exe directory.
pub fn resolve_binary(name: &str, env_var: &str, override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        if p.exists() {
            return p.to_path_buf();
        }
        return p.to_path_buf();
    }
    if let Ok(env) = std::env::var(env_var) {
        return PathBuf::from(env);
    }
    let in_resources = resources_dir().join(name);
    if in_resources.exists() {
        return in_resources;
    }
    let beside_exe = exe_dir().join(name);
    if beside_exe.exists() {
        return beside_exe;
    }
    in_resources
}

pub fn resolve_singbox_exe(override_path: Option<&Path>) -> PathBuf {
    resolve_binary("sing-box.exe", "WIRESENTINEL_SINGBOX_EXE", override_path)
}

pub fn resolve_tor_exe(override_path: Option<&Path>) -> PathBuf {
    resolve_binary("tor.exe", "WIRESENTINEL_TOR_EXE", override_path)
}

pub fn default_tor_data_dir(profile_id: Uuid) -> PathBuf {
    if cfg!(windows) {
        std::env::var("PROGRAMDATA")
            .map(|p| {
                PathBuf::from(p)
                    .join("WireSentinel")
                    .join("tor")
                    .join(profile_id.to_string())
            })
            .unwrap_or_else(|_| {
                PathBuf::from(r"C:\ProgramData\WireSentinel\tor").join(profile_id.to_string())
            })
    } else {
        PathBuf::from("/tmp/WireSentinel/tor").join(profile_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_tor_data_dir_contains_profile_id() {
        let id = Uuid::new_v4();
        let path = default_tor_data_dir(id);
        assert!(path.to_string_lossy().contains(&id.to_string()));
    }
}
