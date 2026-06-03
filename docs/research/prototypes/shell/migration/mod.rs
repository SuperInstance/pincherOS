//! Migration — 3-Phase State Machine with 7 Linguistic Constraints

pub mod guard;

pub use guard::{
    ConsentPolicy, ConsentRecord, ConsentState, DifferentialVerification, FailedReflex,
    MigrationError, MigrationFailureReason, MigrationGuard, MigrationInitiator, MigrationPhase,
    NailFile, PairChannelState, ShellPair, ShapeVerb, SubstanceAccidentPartition,
    SubstanceField, SymbiontTransferStatus, AccidentField,
};
