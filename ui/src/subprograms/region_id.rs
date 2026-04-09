use uuid::*;
use ::serde::*;

///
/// Identifier used to specify a region within the Focus program
///
#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegionId(Uuid);

impl RegionId {
    ///
    /// Creates a unique new region ID
    ///
    pub fn new() -> Self {
        RegionId(Uuid::new_v4())
    }
}
