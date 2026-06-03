//! Sandboxed execution module.

pub mod bwrap;

pub use bwrap::{ExecutionResult, Sandbox, SandboxConfig};
