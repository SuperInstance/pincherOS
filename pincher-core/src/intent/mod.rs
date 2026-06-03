//! Intent contracts — declarative intent-to-action mappings from TOML.
//!
//! This module implements **Intent.toml v2** — a declarative specification
//! system for defining how PincherOS handles classes of intents. Instead of
//! teaching the engine individual intent→action pairs, users can write an
//! `Intent.toml` file that defines patterns, action templates, confidence
//! thresholds, priorities, conflict strategies, and output schemas.
//!
//! # Modules
//!
//! - [`contract`] — Intent contract parsing, validation, and matching
//! - [`schema`] — Lightweight JSON Schema validation for action outputs
//!
//! # Example
//!
//! ```toml
//! [contract]
//! name = "file-operations"
//! confidence_threshold = 0.75
//! priority = 80
//! conflict_strategy = "highest_confidence"
//!
//! [[contract.patterns]]
//! template = "read file {path}"
//! regex = "read\\s+(?:the\\s+)?file\\s+(.+)"
//!
//! [contract.action]
//! template = "file.read {{path}}"
//! ```

pub mod contract;
pub mod schema;

// Re-export primary types at the module level for convenience.
pub use contract::{
    ActionTemplate, ConflictStrategy, ContractError, ContractResult, IntentContract, IntentPattern,
};
pub use schema::{
    FieldType, OutputSchema, SchemaField, SchemaValidationError, SchemaValidationResult,
    SchemaValidator,
};
