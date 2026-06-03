//! Reflex module — intent matching and execution engine
//!
//! The reflex system is the core of PincherOS. It maps intents to actions
//! using vector similarity search, with three tiers of matching:
//! - **Exact**: High-confidence match → short-circuit execution
//! - **Similar**: Moderate match → execute with LLM refinement
//! - **Novel**: No match → learn from LLM guidance

pub mod confidence;
pub mod engine;
pub mod matcher;

// Re-export key types
pub use engine::{
    EngineError, EngineResult, EngineStatus, Execution, MatchType, Reflex, ReflexEngine,
};
pub use matcher::{MatchError, MatchOpResult, MatchResult, MatchThresholds};
