//! Carapace Bridge — WASM sandbox for guest code execution.
//!
//! The carapace is the hard outer surface of the hermit crab's shell that
//! protects the creature inside. In PincherOS, the Carapace Bridge allows
//! third-party and untrusted code to run inside a WASM sandbox with strict
//! capability-based sandboxing.
//!
//! # Architecture
//!
//! - [`guest`] — Guest module management (loading, validating, versioning)
//! - [`host`] — Host functions exposed to guests (file I/O, network, shell)
//! - [`capability`] — Capability gate that enforces permissions at the
//!   host-function boundary
//!
//! # Example
//!
//! ```rust,ignore
//! use pincher_core::carapace::guest::{GuestModule, GuestConfig};
//! use pincher_core::carapace::capability::{CapabilityGate, HostFunction};
//!
//! let module = GuestModule::from_bytes(
//!     wasm_bytes,
//!     "my-plugin",
//!     "1.0.0",
//!     vec!["fs_read".into()],
//! ).unwrap();
//! ```

pub mod capability;
pub mod guest;
pub mod host;

// Re-export primary types at the module level.
pub use capability::{CapabilityGate, GateError, GateResult, HostFunction, SandboxPolicy};
pub use guest::{GuestConfig, GuestError, GuestModule, GuestResult, GuestVersion};
pub use host::{HostError, HostFunctionDispatcher, HostResult};
