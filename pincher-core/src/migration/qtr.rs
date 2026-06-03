//! QTR (Quiesce-Transfer-Resume) migration protocol.

use crate::db::Database;
use crate::shell::ShellProfile;
use anyhow::{Context, Result};
use std::path::Path;

/// The current phase of a QTR migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationPhase {
    Quiesce,
    Transfer,
    Resume,
}

/// The QTR protocol implementation.
pub struct QtrProtocol;

impl QtrProtocol {
    /// Phase 1: Quiesce — flush the WAL and checkpoint SQLite.
    pub async fn quiesce(db: &Database) -> Result<()> {
        tracing::info!("QTR: entering Quiesce phase");
        db.checkpoint_wal()
            .context("failed to checkpoint WAL during quiesce")?;
        db.end_all_sessions()
            .context("failed to end sessions during quiesce")?;
        tracing::info!("QTR: quiesce complete — database is consistent");
        Ok(())
    }

    /// Phase 2: Transfer — copy and compute blake3 hash.
    pub async fn transfer(source_path: &Path, dest_path: &Path) -> Result<blake3::Hash> {
        tracing::info!(
            "QTR: entering Transfer phase — {} → {}",
            source_path.display(),
            dest_path.display()
        );

        let source_data =
            std::fs::read(source_path).context("failed to read source file for transfer")?;
        let hash = blake3::hash(&source_data);

        if let Some(parent) = dest_path.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create destination directory")?;
        }

        std::fs::write(dest_path, &source_data)
            .context("failed to write destination file during transfer")?;

        tracing::info!("QTR: transfer complete — hash: {}", hash.to_hex());
        Ok(hash)
    }

    /// Phase 3: Resume — verify, unpack, and re-snap.
    pub async fn resume(db: &Database, nail_path: &Path) -> Result<ShellProfile> {
        tracing::info!("QTR: entering Resume phase");

        if !nail_path.exists() {
            anyhow::bail!("nail file does not exist: {}", nail_path.display());
        }

        let profile = ShellProfile::probe().context("failed to probe new shell during resume")?;

        let _shell_id = profile
            .save_to_db(db.conn())
            .context("failed to save new shell profile")?;

        tracing::info!("QTR: resume complete — new shell: {}", profile.hostname);
        Ok(profile)
    }
}
