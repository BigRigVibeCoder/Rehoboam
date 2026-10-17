//! Resuming a run from a keyframe through the input journal: the host loop SPR-008's bisect reuses
//! (VER-SPR-007-TEST GAP-10, Architect ruling).
//!
//! The journal position is the number of inputs the engine has **accepted** (`seq` order, applied
//! or still queued), not the number applied. The host checks an input against the engine's
//! cursor, journals it, then submits it, so journal order is acceptance order and a journaled
//! input is never refused. A keyframe at position `p` therefore holds exactly the effects of the
//! journal's first `p` rows: applied ones in its tables, queued ones in its `inputs` table.
//! Resuming re-submits only the rows from `seq` `p` on. Re-submitting from the applied count
//! instead would apply the queued inputs twice.
//!
//! [`resume`] loads the keyframe ([`load_engine`]), checks its anchor against the journal
//! ([`Replay::check_anchor`]), and submits the rows after the position in `seq` order. P0's
//! engine takes material inputs only, so a non-material row is refused (BCK-002 A-33 counts them
//! once they reach the engine).
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §6 item 6; ADR-0025;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-02, T-03.

use std::collections::BTreeSet;
use std::fmt;

use rh_core::engine::{Engine, InputError, MaterialInput};
use rh_core::register::Registered;
use rh_core::time::Calendar;

use crate::journal::{JournalError, JournaledInput, Replay};
use crate::keyframe::{History, KeyframeError, KeyframeHeader, load_engine};

/// Why a run could not be resumed.
#[derive(Debug)]
pub enum ResumeError {
    /// The keyframe was refused.
    Keyframe(KeyframeError),
    /// The journal does not hold the keyframe's anchor.
    Journal(JournalError),
    /// A journal row after the position is not a material input, which P0's engine cannot take.
    NotMaterial {
        /// Its seq.
        seq: u32,
    },
    /// The engine refused a journal row after the position (it is before the keyframe's cursor:
    /// the host broke the check-journal-submit order).
    Input {
        /// Its seq.
        seq: u32,
        /// The refusal.
        error: InputError,
    },
}

impl fmt::Display for ResumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Keyframe(e) => write!(f, "resume: {e}"),
            Self::Journal(e) => write!(f, "resume: {e}"),
            Self::NotMaterial { seq } => {
                write!(
                    f,
                    "resume: journal input {seq} is not material (BCK-002 A-33)"
                )
            }
            Self::Input { seq, error } => write!(f, "resume: journal input {seq}: {error}"),
        }
    }
}

impl std::error::Error for ResumeError {}

/// The engine input of a journaled material input: its scope's entity. `None` for a
/// non-material input.
#[must_use]
pub fn material_input(input: &JournaledInput) -> Option<MaterialInput> {
    let entity = input.scope.entity()?;
    Some(MaterialInput {
        scope: BTreeSet::from([entity]),
    })
}

/// Loads `keyframe` for `expected`, checks its anchor against `journal`, and submits the
/// journal's inputs after the anchor in `seq` order: the engine then runs as the continuous run
/// did (V-DET-05 through the journal).
///
/// # Errors
/// [`ResumeError`]: the keyframe refused, the anchor not in the journal, or a later row that is
/// not material or that the engine refuses.
pub fn resume(
    keyframe: &[u8],
    calendar: Calendar,
    registered: Registered,
    expected: &History,
    journal: &Replay,
) -> Result<(Engine, KeyframeHeader), ResumeError> {
    let (mut engine, header) =
        load_engine(keyframe, calendar, registered, expected).map_err(ResumeError::Keyframe)?;
    journal
        .check_anchor(&header.journal)
        .map_err(ResumeError::Journal)?;
    let mut after: Vec<&JournaledInput> = journal
        .inputs()
        .iter()
        .filter(|i| i.seq >= header.journal.inputs)
        .collect();
    after.sort_by_key(|i| i.seq);
    for input in after {
        let material = material_input(input).ok_or(ResumeError::NotMaterial { seq: input.seq })?;
        engine
            .submit_input(input.tick, material)
            .map_err(|error| ResumeError::Input {
                seq: input.seq,
                error,
            })?;
    }
    Ok((engine, header))
}
