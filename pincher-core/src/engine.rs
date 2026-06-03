//! ReflexEngine — the core engine for storing, matching, and executing reflexes.
//!
//! The hermit crab metaphor: each reflex is a "shell" the OS can inhabit.
//! The engine finds the best-fitting shell for any given intent.

use crate::embedder::Embedder;
use crate::types::{DbStats, NailFile, Reflex, ShellFingerprint};
use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use tracing::{debug, info, instrument};

/// The minimum cosine similarity threshold for a match.
const MATCH_THRESHOLD: f64 = 0.3;

/// The ReflexEngine stores and retrieves intent-action pairs (reflexes).
pub struct ReflexEngine {
    db: Connection,
    embedder: Embedder,
}

impl ReflexEngine {
    /// Create a new ReflexEngine, opening/creating the database at the given path.
    #[instrument(skip(embedder))]
    pub fn new(db_path: &Path, embedder: Embedder) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating db directory {:?}", parent))?;
        }

        let db = Connection::open(db_path)
            .with_context(|| format!("opening database at {:?}", db_path))?;

        let engine = Self { db, embedder };
        engine.init_db()?;
        info!("ReflexEngine initialized at {:?}", db_path);
        Ok(engine)
    }

    /// Initialize the database schema.
    fn init_db(&self) -> Result<()> {
        self.db
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS reflexes (
                    id            INTEGER PRIMARY KEY AUTOINCREMENT,
                    intent        TEXT NOT NULL,
                    action        TEXT NOT NULL,
                    embedding     BLOB NOT NULL,
                    confidence    REAL NOT NULL DEFAULT 0.0,
                    created_at    TEXT NOT NULL,
                    invoked_count INTEGER NOT NULL DEFAULT 0
                );

                CREATE INDEX IF NOT EXISTS idx_reflexes_intent ON reflexes(intent);

                CREATE TABLE IF NOT EXISTS meta (
                    key   TEXT PRIMARY KEY,
                    value TEXT NOT NULL
                );
                ",
            )
            .context("creating database schema")?;
        Ok(())
    }

    /// Teach the engine a new reflex: associate an intent with an action.
    #[instrument(skip(self))]
    pub async fn teach(&self, intent: &str, action: &str) -> Result<Reflex> {
        let embedding = self.embedder.embed(intent).await?;
        let now = Utc::now().to_rfc3339();
        let emb_bytes = Self::embed_to_bytes(&embedding);

        self.db.execute(
            "INSERT INTO reflexes (intent, action, embedding, confidence, created_at, invoked_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![intent, action, emb_bytes, 1.0, now, 0],
        ).context("inserting reflex")?;

        let id = self.db.last_insert_rowid();
        info!(id, intent, action, "taught new reflex");

        // Update confidence based on similarity to existing reflexes
        let updated_confidence = self.recompute_confidence(&embedding)?;

        self.db.execute(
            "UPDATE reflexes SET confidence = ?1 WHERE id = ?2",
            rusqlite::params![updated_confidence, id],
        ).context("updating confidence")?;

        Ok(Reflex {
            id,
            intent: intent.to_string(),
            action: action.to_string(),
            confidence: updated_confidence,
            created_at: now,
            invoked_count: 0,
        })
    }

    /// Find the best-matching reflex for a given intent.
    #[instrument(skip(self))]
    pub async fn match_reflex(&self, intent: &str) -> Result<Option<Reflex>> {
        let query_embedding = self.embedder.embed(intent).await?;
        let mut best: Option<(Reflex, f64)> = None;

        let mut stmt = self
            .db
            .prepare("SELECT id, intent, action, embedding, confidence, created_at, invoked_count FROM reflexes")
            .context("preparing select for match")?;

        let rows = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let intent: String = row.get(1)?;
                let action: String = row.get(2)?;
                let emb_blob: Vec<u8> = row.get(3)?;
                let confidence: f64 = row.get(4)?;
                let created_at: String = row.get(5)?;
                let invoked_count: i64 = row.get(6)?;
                Ok((id, intent, action, emb_blob, confidence, created_at, invoked_count))
            })
            .context("querying reflexes for match")?;

        for row_result in rows {
            let (id, intent_str, action, emb_blob, confidence, created_at, invoked_count) =
                row_result?;
            let stored_embedding = Self::bytes_to_embed(&emb_blob)?;
            let similarity = Embedder::cosine_similarity(&query_embedding, &stored_embedding);

            debug!(id, similarity, intent = %intent_str, "comparing reflex");

            if similarity >= MATCH_THRESHOLD {
                match &best {
                    Some((_, best_sim)) if similarity <= *best_sim => {}
                    _ => {
                        best = Some((
                            Reflex {
                                id,
                                intent: intent_str,
                                action,
                                confidence,
                                created_at,
                                invoked_count,
                            },
                            similarity,
                        ));
                    }
                }
            }
        }

        if let Some((reflex, sim)) = &best {
            info!(id = reflex.id, similarity = sim, "matched reflex");
        } else {
            info!("no matching reflex found");
        }

        Ok(best.map(|(r, _)| r))
    }

    /// Execute a natural language intent: find the best match and return its action.
    #[instrument(skip(self))]
    pub async fn do_command(&self, intent: &str) -> Result<Option<String>> {
        let matched = self.match_reflex(intent).await?;

        if let Some(reflex) = &matched {
            // Increment invoked count
            self.db.execute(
                "UPDATE reflexes SET invoked_count = invoked_count + 1 WHERE id = ?1",
                rusqlite::params![reflex.id],
            ).context("updating invoked count")?;

            info!(id = reflex.id, action = %reflex.action, "executing reflex");
            Ok(Some(reflex.action.clone()))
        } else {
            info!("no reflex matched for execution");
            Ok(None)
        }
    }

    /// List all stored reflexes.
    pub fn list_reflexes(&self) -> Result<Vec<Reflex>> {
        let mut stmt = self
            .db
            .prepare("SELECT id, intent, action, confidence, created_at, invoked_count FROM reflexes ORDER BY id")
            .context("preparing list query")?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Reflex {
                    id: row.get(0)?,
                    intent: row.get(1)?,
                    action: row.get(2)?,
                    confidence: row.get(3)?,
                    created_at: row.get(4)?,
                    invoked_count: row.get(5)?,
                })
            })
            .context("querying reflexes list")?;

        let mut reflexes = Vec::new();
        for row in rows {
            reflexes.push(row?);
        }
        Ok(reflexes)
    }

    /// Return the number of stored reflexes.
    pub fn reflex_count(&self) -> Result<usize> {
        let count: i64 = self
            .db
            .query_row("SELECT COUNT(*) FROM reflexes", [], |row| row.get(0))
            .context("counting reflexes")?;
        Ok(count as usize)
    }

    /// Pack the current state into a .nail file for migration.
    #[instrument(skip(self))]
    pub fn pack(&self, output: &Path) -> Result<()> {
        let reflexes = self.list_reflexes()?;
        let fingerprint = self.shell_fingerprint();

        let nail = NailFile {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: Utc::now().to_rfc3339(),
            shell_fingerprint: fingerprint,
            reflexes,
        };

        let json = serde_json::to_string_pretty(&nail).context("serializing nail file")?;
        fs::write(output, json).context("writing nail file")?;

        info!(path = ?output, "packed state into nail file");
        Ok(())
    }

    /// Unpack a .nail file and merge its state into the database.
    #[instrument(skip(self))]
    pub async fn unpack(&self, nail_path: &Path) -> Result<()> {
        let content = fs::read_to_string(nail_path).context("reading nail file")?;
        let nail: NailFile = serde_json::from_str(&content).context("parsing nail file")?;

        for reflex in &nail.reflexes {
            // Check if reflex already exists (by intent)
            let exists: bool = self
                .db
                .query_row(
                    "SELECT COUNT(*) > 0 FROM reflexes WHERE intent = ?1",
                    rusqlite::params![reflex.intent],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if !exists {
                // Re-embed the intent to populate the embedding column
                let embedding = self.embedder.embed(&reflex.intent).await
                    .unwrap_or_else(|_| vec![0.0f32; crate::embedder::EMBED_DIM]);
                let emb_bytes = Self::embed_to_bytes(&embedding);

                self.db.execute(
                    "INSERT INTO reflexes (intent, action, embedding, confidence, created_at, invoked_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![
                        reflex.intent,
                        reflex.action,
                        emb_bytes,
                        reflex.confidence,
                        reflex.created_at,
                        reflex.invoked_count,
                    ],
                ).context("inserting unpacked reflex")?;
                info!(intent = %reflex.intent, "imported reflex from nail");
            } else {
                debug!(intent = %reflex.intent, "skipping existing reflex");
            }
        }

        info!(path = ?nail_path, count = nail.reflexes.len(), "unpacked nail file");
        Ok(())
    }

    /// Get the shell fingerprint for the current system.
    pub fn shell_fingerprint(&self) -> ShellFingerprint {
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        let os = std::env::consts::OS.to_string();
        let arch = std::env::consts::ARCH.to_string();

        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);

        // Memory info from /proc/meminfo on Linux, or sensible defaults
        let (total_ram_gb, ram_usage_percent) = Self::read_mem_info();

        ShellFingerprint {
            hostname,
            os,
            arch,
            cpu_cores,
            total_ram_gb,
            ram_usage_percent,
        }
    }

    /// Get database statistics.
    pub fn db_stats(&self) -> DbStats {
        let reflex_count = self.reflex_count().unwrap_or(0);
        let db_size_bytes = self
            .db
            .path()
            .map(|p| fs::metadata(p).map(|m| m.len()).unwrap_or(0))
            .unwrap_or(0);

        DbStats {
            reflex_count,
            db_size_bytes,
        }
    }

    /// Return a reference to the embedder.
    pub fn embedder(&self) -> &Embedder {
        &self.embedder
    }

    // --- Private helpers ---

    fn recompute_confidence(&self, embedding: &[f32]) -> Result<f64> {
        // Confidence is based on how distinct this reflex is from others.
        // If it's very similar to existing reflexes, lower confidence.
        // If it's unique, high confidence.
        let mut max_similarity = 0.0f64;

        let mut stmt = self
            .db
            .prepare("SELECT embedding FROM reflexes")
            .context("preparing confidence query")?;

        let rows = stmt
            .query_map([], |row| {
                let emb_blob: Vec<u8> = row.get(0)?;
                Ok(emb_blob)
            })
            .context("querying embeddings for confidence")?;

        for row_result in rows {
            let emb_blob = row_result?;
            if let Ok(stored) = Self::bytes_to_embed(&emb_blob) {
                let sim = Embedder::cosine_similarity(embedding, &stored);
                if sim > max_similarity {
                    max_similarity = sim;
                }
            }
        }

        // Confidence: 1.0 if unique, decreases with similarity to others
        let confidence = (1.0 - max_similarity * 0.5).max(0.1).min(1.0);
        Ok(confidence)
    }

    fn embed_to_bytes(embedding: &[f32]) -> Vec<u8> {
        embedding
            .iter()
            .flat_map(|f| f.to_le_bytes())
            .collect()
    }

    fn bytes_to_embed(bytes: &[u8]) -> Result<Vec<f32>> {
        if bytes.len() % 4 != 0 {
            anyhow::bail!("invalid embedding blob length: {}", bytes.len());
        }
        let mut result = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            result.push(f);
        }
        Ok(result)
    }

    fn read_mem_info() -> (f64, f64) {
        // Try reading /proc/meminfo on Linux
        if let Ok(content) = fs::read_to_string("/proc/meminfo") {
            let mut total_kb: f64 = 0.0;
            let mut available_kb: f64 = 0.0;

            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line
                        .split_whitespace()
                        .nth(1)
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                }
            }

            if total_kb > 0.0 {
                let total_gb = total_kb / 1024.0 / 1024.0;
                let used_percent = if total_kb > 0.0 {
                    ((total_kb - available_kb) / total_kb) * 100.0
                } else {
                    0.0
                };
                return (total_gb, used_percent);
            }
        }

        // Fallback for non-Linux systems
        (8.0, 50.0)
    }
}
