//! Capability manifest — declares what a reflex is allowed to do.
//!
//! A [`CapabilityManifest`] is attached to each reflex and enumerates the
//! permissions it requires.  The veto engine checks the manifest before
//! allowing execution.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single permission granted to a reflex.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    /// Read access to a filesystem path (may include globs).
    FsRead {
        /// Path pattern this permission covers.
        path: String,
    },
    /// Write access to a filesystem path.
    FsWrite {
        /// Path pattern this permission covers.
        path: String,
    },
    /// Outbound network connection to a specific host:port.
    NetConnect {
        /// Hostname or IP address.
        host: String,
        /// TCP/UDP port.
        port: u16,
    },
    /// No network access at all.
    NetNone,
    /// Permission to execute a specific binary.
    Execute {
        /// Name or path of the binary.
        binary: String,
    },
}

impl Permission {
    /// Returns `true` if this permission grants filesystem write access
    /// that covers `path`.
    pub fn covers_fs_write(&self, path: &str) -> bool {
        match self {
            Permission::FsWrite { path: pattern } => path_matches_pattern(pattern, path),
            _ => false,
        }
    }

    /// Returns `true` if this permission grants filesystem read access
    /// that covers `path`.
    pub fn covers_fs_read(&self, path: &str) -> bool {
        match self {
            Permission::FsRead { path: pattern } => path_matches_pattern(pattern, path),
            // Write implies read.
            Permission::FsWrite { path: pattern } => path_matches_pattern(pattern, path),
            _ => false,
        }
    }

    /// Returns `true` if this permission allows executing `binary`.
    pub fn covers_execute(&self, binary: &str) -> bool {
        match self {
            Permission::Execute { binary: allowed } => allowed == binary,
            _ => false,
        }
    }

    /// Returns `true` if this permission allows network connections.
    pub fn allows_network(&self) -> bool {
        matches!(self, Permission::NetConnect { .. })
    }
}

/// A manifest declaring the permissions and required capabilities for a reflex.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityManifest {
    /// The reflex this manifest is attached to.
    pub reflex_id: Uuid,
    /// Permissions granted to this reflex.
    pub permissions: Vec<Permission>,
    /// Named capabilities required by this reflex (e.g. "cuda", "gpu").
    pub required_capabilities: Vec<String>,
}

impl CapabilityManifest {
    /// Create an empty manifest for the given reflex.
    pub fn empty(reflex_id: Uuid) -> Self {
        Self {
            reflex_id,
            permissions: vec![],
            required_capabilities: vec![],
        }
    }

    /// Returns `true` if any permission in this manifest grants fs:write
    /// access to `path`.
    pub fn has_fs_write(&self, path: &str) -> bool {
        self.permissions.iter().any(|p| p.covers_fs_write(path))
    }

    /// Returns `true` if any permission in this manifest grants fs:read
    /// access to `path`.
    pub fn has_fs_read(&self, path: &str) -> bool {
        self.permissions.iter().any(|p| p.covers_fs_read(path))
    }

    /// Returns `true` if any permission allows executing `binary`.
    pub fn has_execute(&self, binary: &str) -> bool {
        self.permissions.iter().any(|p| p.covers_execute(binary))
    }

    /// Returns `true` if any permission allows network access.
    pub fn allows_network(&self) -> bool {
        self.permissions.iter().any(|p| p.allows_network())
    }
}

/// Simple glob-style path matching.
///
/// Supports `*` as a wildcard that matches any sequence of characters.
fn path_matches_pattern(pattern: &str, path: &str) -> bool {
    if pattern == "/*" || pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return pattern == path || path.starts_with(pattern);
    }
    // Very simple glob: split on '*' and check sequential containment.
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    let mut cursor = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(pos) = path[cursor..].find(part) {
            if i == 0 && pos != 0 && !pattern.starts_with('*') {
                return false;
            }
            cursor += pos + part.len();
        } else {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matching_exact() {
        assert!(path_matches_pattern("/tmp", "/tmp"));
        assert!(!path_matches_pattern("/tmp", "/var"));
    }

    #[test]
    fn test_path_matching_wildcard() {
        assert!(path_matches_pattern("/*", "/anything"));
        assert!(path_matches_pattern("/tmp/*", "/tmp/foo"));
    }

    #[test]
    fn test_permission_covers() {
        let p = Permission::FsWrite {
            path: "/tmp/*".into(),
        };
        assert!(p.covers_fs_write("/tmp/test.txt"));
        assert!(!p.covers_fs_write("/etc/shadow"));
    }
}
