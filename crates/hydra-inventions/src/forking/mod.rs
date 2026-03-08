pub mod compare;
pub mod fork;
pub mod merge;
pub mod parallel;

pub use compare::{ComparisonResult, OutcomeComparator};
pub use fork::{ForkBranch, ForkPoint};
pub use merge::{MergeStrategy, MergedResult, ResultMerger};
pub use parallel::{BranchResult, ParallelExecutor};
