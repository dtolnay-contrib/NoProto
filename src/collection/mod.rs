//! Collections: NP_Table, NP_Tuple, NP_List & NP_Map

use crate::{error::NP_Error, pointer::NP_Ptr};

/// Table data type
pub mod table;
/// Map data type
pub mod map;
/// List data type
pub mod list;
/// Tuple data type
pub mod tuple;

#[doc(hidden)]
pub trait NP_Collection<'collection> {
    /// Get length of collection
    fn length(&self) -> usize;
    /// Step a pointer to the next item in the collection
    fn step_pointer(ptr: &mut NP_Ptr<'collection>) -> Option<NP_Ptr<'collection>>;
    /// Commit a virtual pointer into the buffer
    fn commit_pointer(ptr: NP_Ptr<'collection>) -> Result<NP_Ptr<'collection>, NP_Error>;
}