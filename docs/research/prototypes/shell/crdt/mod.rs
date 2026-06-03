//! CRDT Engine — Partitioned hot/cold merge

pub mod engine;

pub use engine::{
    CrdtCell, CrdtCellType, CrdtError, MergeResult, MergeStats, PartitionConfig,
    PartitionedCrdtEngine, PendingUpdate, CellTemperature,
};
