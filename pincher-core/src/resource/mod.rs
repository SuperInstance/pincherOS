//! Resource monitoring and management module

pub mod controller;
pub mod pid;

// Re-export key types from controller (the main one)
pub use controller::{
    PidController, ResourceBudget, ResourceController, ResourceError, ResourceMetrics,
    ResourceResult, ResourceState, ResourceThresholds,
};
