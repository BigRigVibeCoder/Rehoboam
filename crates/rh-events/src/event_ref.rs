//! `EventRef`: a reference to an event in a draft's, timer's or command's slot (EVO-001 §2; `05.1` §2 rule 2; envelope §2).

use crate::{DraftRef, EventId};

/// An event a slot refers to. The merge resolves `Draft` to the id it assigns; there is no
/// integer conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventRef {
    /// One of the step's own earlier drafts.
    Draft(DraftRef),
    /// An id read from state (a timer's or command's `cause`, a column).
    Known(EventId),
    /// No event.
    None,
}
