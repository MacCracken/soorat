//! DRM device information — kernel-level GPU details.
//!
//! Provides driver version, device enumeration, display connector info, and
//! DRM capabilities. With the `drm` feature enabled, queries the kernel via
//! agnosys. Without it, all functions return [`RenderError`] or empty results.
//!
//! This is **complementary** to wgpu — wgpu abstracts the GPU for rendering,
//! while this module exposes Linux-specific kernel information useful for
//! device selection UIs, diagnostics, and buffer sharing queries.

use crate::error::{RenderError, Result};
use std::path::PathBuf;

/// DRM driver version.
#[derive(Debug, Clone)]
pub struct DriverVersion {
    pub major: i32,
    pub minor: i32,
    pub patchlevel: i32,
    pub name: String,
    pub date: String,
    pub description: String,
}

/// Summary of a DRM card device.
#[derive(Debug, Clone)]
pub struct CardInfo {
    /// Device path (e.g., `/dev/dri/card0`).
    pub path: PathBuf,
    /// Driver version (if queryable).
    pub driver: Option<DriverVersion>,
}

/// Display connector summary.
#[derive(Debug, Clone)]
pub struct ConnectorSummary {
    pub id: u32,
    pub connector_type: String,
    pub connected: bool,
    pub mm_width: u32,
    pub mm_height: u32,
}

/// List DRM card devices with driver info.
///
/// Returns one [`CardInfo`] per `/dev/dri/card*` device. With the `drm`
/// feature disabled or on non-Linux, returns an empty vec.
#[must_use]
pub fn list_cards() -> Vec<CardInfo> {
    tracing::debug!("listing drm cards");
    #[cfg(feature = "drm")]
    {
        _list_cards_impl()
    }
    #[cfg(not(feature = "drm"))]
    {
        Vec::new()
    }
}

#[cfg(feature = "drm")]
fn _list_cards_impl() -> Vec<CardInfo> {
    let paths = match agnosys::drm::enumerate_cards() {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };

    paths
        .into_iter()
        .map(|path| {
            let driver = agnosys::drm::Device::open(&path)
                .and_then(|dev| dev.version())
                .ok()
                .map(|v| DriverVersion {
                    major: v.major,
                    minor: v.minor,
                    patchlevel: v.patchlevel,
                    name: v.name,
                    date: v.date,
                    description: v.desc,
                });
            CardInfo { path, driver }
        })
        .collect()
}

/// List render node paths (e.g., `/dev/dri/renderD128`).
///
/// With the `drm` feature disabled or on non-Linux, returns an empty vec.
#[must_use]
pub fn list_render_nodes() -> Vec<PathBuf> {
    #[cfg(feature = "drm")]
    {
        agnosys::drm::enumerate_render_nodes().unwrap_or_default()
    }
    #[cfg(not(feature = "drm"))]
    {
        Vec::new()
    }
}

/// Query driver version for a specific card path.
///
/// Requires the `drm` feature. Without it, always returns an error.
pub fn driver_version(card_path: &std::path::Path) -> Result<DriverVersion> {
    #[cfg(feature = "drm")]
    {
        let dev = agnosys::drm::Device::open(card_path)
            .map_err(|e| RenderError::DeviceRequest(format!("DRM open failed: {e}")))?;
        let v = dev
            .version()
            .map_err(|e| RenderError::DeviceRequest(format!("DRM version query failed: {e}")))?;
        Ok(DriverVersion {
            major: v.major,
            minor: v.minor,
            patchlevel: v.patchlevel,
            name: v.name,
            date: v.date,
            description: v.desc,
        })
    }
    #[cfg(not(feature = "drm"))]
    {
        let _ = card_path;
        Err(RenderError::DeviceRequest(
            "DRM support requires the 'drm' feature".into(),
        ))
    }
}

/// Query display connectors for a card.
///
/// Requires the `drm` feature. Without it, always returns an error.
pub fn list_connectors(card_path: &std::path::Path) -> Result<Vec<ConnectorSummary>> {
    #[cfg(feature = "drm")]
    {
        let dev = agnosys::drm::Device::open(card_path)
            .map_err(|e| RenderError::DeviceRequest(format!("DRM open failed: {e}")))?;
        let res = dev
            .mode_resources()
            .map_err(|e| RenderError::DeviceRequest(format!("DRM mode resources failed: {e}")))?;

        let mut connectors = Vec::new();
        for &id in &res.connector_ids {
            if let Ok(info) = dev.connector_info(id) {
                connectors.push(ConnectorSummary {
                    id: info.id,
                    connector_type: format!("{:?}", info.connector_type),
                    connected: info.status == agnosys::drm::ConnectionStatus::Connected,
                    mm_width: info.mm_width,
                    mm_height: info.mm_height,
                });
            }
        }
        Ok(connectors)
    }
    #[cfg(not(feature = "drm"))]
    {
        let _ = card_path;
        Err(RenderError::DeviceRequest(
            "DRM support requires the 'drm' feature".into(),
        ))
    }
}

/// Check if PRIME (GPU buffer sharing) is supported on a card.
///
/// Requires the `drm` feature. Without it, returns `false`.
#[must_use]
pub fn supports_prime(card_path: &std::path::Path) -> bool {
    #[cfg(feature = "drm")]
    {
        agnosys::drm::Device::open(card_path)
            .and_then(|dev| dev.get_cap(agnosys::drm::Cap::Prime))
            .map(|v| v != 0)
            .unwrap_or(false)
    }
    #[cfg(not(feature = "drm"))]
    {
        let _ = card_path;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_cards_returns_vec() {
        // May be empty (no GPU, CI, or non-Linux) — just verify no panic
        let cards = list_cards();
        let _ = cards;
    }

    #[test]
    fn list_render_nodes_returns_vec() {
        let nodes = list_render_nodes();
        let _ = nodes;
    }

    #[test]
    fn driver_version_nonexistent_path() {
        let result = driver_version(std::path::Path::new("/dev/dri/card999"));
        assert!(result.is_err());
    }

    #[test]
    fn list_connectors_nonexistent_path() {
        let result = list_connectors(std::path::Path::new("/dev/dri/card999"));
        assert!(result.is_err());
    }

    #[test]
    fn supports_prime_nonexistent_path() {
        assert!(!supports_prime(std::path::Path::new("/dev/dri/card999")));
    }

    #[test]
    fn card_info_debug() {
        let info = CardInfo {
            path: PathBuf::from("/dev/dri/card0"),
            driver: Some(DriverVersion {
                major: 1,
                minor: 2,
                patchlevel: 3,
                name: "test".into(),
                date: "20260324".into(),
                description: "Test driver".into(),
            }),
        };
        let dbg = format!("{:?}", info);
        assert!(dbg.contains("card0"));
        assert!(dbg.contains("test"));
    }

    #[test]
    fn card_info_no_driver() {
        let info = CardInfo {
            path: PathBuf::from("/dev/dri/card0"),
            driver: None,
        };
        assert!(info.driver.is_none());
    }

    #[test]
    fn connector_summary_debug() {
        let conn = ConnectorSummary {
            id: 42,
            connector_type: "HDMIA".into(),
            connected: true,
            mm_width: 530,
            mm_height: 300,
        };
        let dbg = format!("{:?}", conn);
        assert!(dbg.contains("HDMIA"));
        assert!(dbg.contains("42"));
    }

    #[test]
    fn driver_version_clone() {
        let v = DriverVersion {
            major: 5,
            minor: 15,
            patchlevel: 0,
            name: "amdgpu".into(),
            date: "20260101".into(),
            description: "AMD GPU driver".into(),
        };
        let cloned = v.clone();
        assert_eq!(cloned.name, "amdgpu");
        assert_eq!(cloned.major, 5);
    }
}
