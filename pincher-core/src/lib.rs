//! # PincherOS Core Library
//!
//! `pincher-core` is the core library for PincherOS — a "post-model operating
//! system" using the hermit crab metaphor.
//!
//! ## Architecture
//!
//! The codebase is transitioning from a legacy architecture to a unified one.
//! The "new" architecture (in `reflex/`, `embed/`, `db/`, `resource/`) is
//! the canonical path. The "old" architecture (`engine.rs`, `embedder.rs`,
//! `types.rs`) is deprecated and will be removed once the CLI is migrated.
//!
//! **Use `reflex::ReflexEngine` (new), NOT `engine::ReflexEngine` (old).**

pub mod capability;
pub mod carapace;
pub mod db;
pub mod dynamics;
pub mod embed;
pub mod embedder;  // DEPRECATED — use embed::Embedder instead
pub mod engine;    // DEPRECATED — use reflex::ReflexEngine instead
pub mod immunology;
pub mod intent;
pub mod migration;
pub mod reflex;
pub mod resource;
pub mod rpc;
pub mod sandbox;
pub mod security;
pub mod shell;
pub mod types;     // DEPRECATED — types moved to their respective modules

// ── Crate-level re-exports ──────────────────────────────────────────

// Primary (NEW) engine & embedder — use these
pub use embed::{
    EmbedError, EmbedResult,
    cosine_similarity,
    EMBEDDING_DIM,
    download_model,
};

// Reflex subsystem (primary)
pub use reflex::{
    EngineError, EngineResult, EngineStatus, Execution, MatchType,
    MatchError, MatchThresholds,
    Reflex, ReflexEngine,  // NEW: Use this ReflexEngine
};

// Database
pub use db::{
    Database, DbError, DbResult,
    schema::{
        ActionLogRow, ReflexRow, SessionRow, ShellRow,
        EMBEDDING_DIM as DB_EMBEDDING_DIM,
        embed_to_bytes, bytes_to_embed,
    },
};

// Resource
pub use resource::{
    PidController, ResourceBudget, ResourceController as ResourceCtrl, ResourceError, ResourceMetrics,
    ResourceResult, ResourceState, ResourceThresholds,
};

// Security
pub use security::{
    Capability as SecCapability, LandlockRule, SandboxConfig, SandboxError as SecSandboxError,
    SandboxResult as SecSandboxResult, SignedToken,
    veto::{VetoDecision, VetoEngine as SecVetoEngine, VetoError, VetoResult, VetoRule, ExecutionContext},
};

// Capability
pub use capability::{
    manifest::{CapabilityManifest, Permission},
    token::CapabilityToken,
};

// Migration
pub use migration::{
    compatibility_score, fingerprint, fingerprint_hash,
    FingerprintError, FingerprintResult,
    pack_nail, unpack_nail, verify_nail, read_manifest, read_identity,
    AgentConfig, AgentIdentity, AgentPreferences, NailChecksums, NailManifest,
    PackError, PackResult,
    ShellFingerprint,  // NEW: canonical ShellFingerprint
};

// RPC
pub use rpc::{
    start_rpc_server, EngineCommand, JsonRpcRequest, JsonRpcResponse, RpcError, RpcErrorValue,
    RpcRequest, RpcResponse,
};

// ── Legacy (DEPRECATED) re-exports ─────────────────────────────────
// These exist only for backward compatibility with pincher-cli.
// DO NOT use in new code.

#[deprecated(since = "0.2.0", note = "Use embed::Embedder instead")]
pub use embedder::{Embedder as LegacyEmbedder, EMBED_DIM};

#[deprecated(since = "0.2.0", note = "Use reflex::ReflexEngine instead")]
pub use engine::ReflexEngine as LegacyReflexEngine;

#[deprecated(since = "0.2.0", note = "Use reflex::Reflex and migration::ShellFingerprint instead")]
pub use types::{DbStats, NailFile, Reflex as LegacyReflex, ShellFingerprint as LegacyShellFingerprint};
