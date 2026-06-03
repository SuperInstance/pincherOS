//! NailUnpack — verifies and extracts `.nail` archives.

use super::pack::NailManifest;
use anyhow::{Context, Result};
use std::path::Path;

/// The NailUnpack extractor.
pub struct NailUnpack;

impl NailUnpack {
    /// Unpack a `.nail` archive into `target_dir`.
    pub fn unpack(nail_path: &Path, target_dir: &Path) -> Result<NailManifest> {
        tracing::info!(
            "NailUnpack: starting unpack — {} → {}",
            nail_path.display(),
            target_dir.display()
        );

        let compressed =
            std::fs::read(nail_path).context("failed to read nail archive")?;
        let tar_data = zstd::bulk::decompress(&compressed, 64 * 1024 * 1024)
            .context("failed to decompress nail archive")?;

        std::fs::create_dir_all(target_dir)
            .context("failed to create target directory")?;

        let mut archive = tar::Archive::new(std::io::Cursor::new(&tar_data));
        archive
            .unpack(target_dir)
            .context("failed to extract tar archive")?;

        let manifest_path = target_dir.join("manifest.json");
        let manifest_data =
            std::fs::read_to_string(&manifest_path).context("failed to read manifest.json")?;
        let manifest: NailManifest =
            serde_json::from_str(&manifest_data).context("failed to parse manifest.json")?;

        // Verify database checksum
        let db_path = target_dir.join("reflexes.db");
        if db_path.exists() {
            let db_data = std::fs::read(&db_path).context("failed to read reflexes.db")?;
            let db_checksum = blake3::hash(&db_data).to_hex().to_string();
            if db_checksum != manifest.checksums.reflexes_db {
                anyhow::bail!(
                    "database checksum mismatch: expected {}, got {}",
                    manifest.checksums.reflexes_db,
                    db_checksum
                );
            }
        }

        // Verify config checksum
        let config_path = target_dir.join("config.toml");
        if config_path.exists() {
            let config_data =
                std::fs::read(&config_path).context("failed to read config.toml")?;
            let config_checksum = blake3::hash(&config_data).to_hex().to_string();
            if config_checksum != manifest.checksums.config_toml {
                anyhow::bail!(
                    "config checksum mismatch: expected {}, got {}",
                    manifest.checksums.config_toml,
                    config_checksum
                );
            }
        }

        // Validate the SQLite schema
        if db_path.exists() {
            validate_db_schema(&db_path)?;
        }

        tracing::info!(
            "NailUnpack: unpack complete — {} reflexes",
            manifest.reflex_count
        );

        Ok(manifest)
    }
}

fn validate_db_schema(db_path: &Path) -> Result<()> {
    let conn = rusqlite::Connection::open(db_path)
        .context("failed to open database for schema validation")?;

    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
        .and_then(|mut stmt| {
            stmt.query_map([], |row| row.get(0))
                .map(|rows| rows.filter_map(|r| r.ok()).collect())
        })
        .context("failed to query sqlite_master")?;

    let required = ["reflexes", "sessions", "shells"];
    for table in &required {
        if !tables.iter().any(|t| t == *table) {
            anyhow::bail!("missing required table: {}", table);
        }
    }

    tracing::info!("database schema validation passed");
    Ok(())
}
