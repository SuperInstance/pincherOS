//! Capability module for PincherOS

pub mod manifest;
pub mod token;

pub use manifest::{CapabilityManifest, Permission};
pub use token::CapabilityToken;
