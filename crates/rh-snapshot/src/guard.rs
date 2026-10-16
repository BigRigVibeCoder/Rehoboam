//! The unsynced guard: read-only APIs never change plane A (SPR-007 T-04; DET-11; V-DET-10).
//!
//! Read-only consumers (frame builders, the Python `frame()`, LLM context builders, invariant
//! checks) get immutable references, so safe Rust already forbids a direct write. What the borrow
//! checker cannot see is a write through interior mutability (a cache in a `Mutex`, an atomic, a
//! foreign-function call) reaching state that plane A hashes. [`read_only`] closes that gap at run
//! time: it hashes plane A before and after the call and reports any change as
//! [`UnsyncedWrite`].
//!
//! The check runs in debug builds, which is what CI tests (`05.1` §6 item 7), and is compiled out
//! of release builds: there [`read_only`] only calls the API ([`GUARD_ENABLED`] says which).
//! Every host entry point that serves a read-only API wraps it in [`read_only`].
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §6 item 7; `docs/09` V-DET-10; ADR-0024;
//! `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-04, ruling 5.

use std::fmt;

use rh_core::engine::Engine;
use rh_core::world::WorldTables;

use crate::hash::tables_hash;
use crate::state::state_hash;

/// Whether this build checks read-only calls (debug builds) or compiles the check out (release).
pub const GUARD_ENABLED: bool = cfg!(debug_assertions);

/// State whose plane-A hash the guard compares.
pub trait PlaneAState {
    /// The plane-A hash of the state (`state_hash`).
    fn plane_a_hash(&self) -> u64;
}

/// Bare tables: their [`tables_hash`].
impl PlaneAState for WorldTables {
    fn plane_a_hash(&self) -> u64 {
        tables_hash(self)
    }
}

/// An engine: `state_hash` of its snapshot, the engine-held tables included (VER-SPR-007 GAP-4).
impl PlaneAState for Engine {
    fn plane_a_hash(&self) -> u64 {
        state_hash(&self.snapshot())
    }
}

/// A read-only API changed plane A.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsyncedWrite {
    /// The API that wrote.
    pub api: &'static str,
    /// The plane-A hash before the call.
    pub before: u64,
    /// The plane-A hash after it.
    pub after: u64,
}

impl fmt::Display for UnsyncedWrite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unsynced guard: read-only API {:?} changed plane A ({:016x} -> {:016x})",
            self.api, self.before, self.after
        )
    }
}

impl std::error::Error for UnsyncedWrite {}

/// Calls the read-only API `call` on `state`; in debug builds, checks that plane A is unchanged.
///
/// # Errors
/// [`UnsyncedWrite`] if the plane-A hash after the call differs from the hash before it (debug
/// builds only; release builds never return it).
#[cfg(debug_assertions)]
pub fn read_only<S: PlaneAState + ?Sized, R>(
    api: &'static str,
    state: &S,
    call: impl FnOnce(&S) -> R,
) -> Result<R, UnsyncedWrite> {
    let before = state.plane_a_hash();
    let result = call(state);
    let after = state.plane_a_hash();
    if before == after {
        Ok(result)
    } else {
        Err(UnsyncedWrite { api, before, after })
    }
}

/// Calls the read-only API `call` on `state`. Release build: the guard is compiled out.
///
/// # Errors
/// Never in release builds; the signature matches the debug build's.
#[cfg(not(debug_assertions))]
#[inline]
pub fn read_only<S: PlaneAState + ?Sized, R>(
    _api: &'static str,
    state: &S,
    call: impl FnOnce(&S) -> R,
) -> Result<R, UnsyncedWrite> {
    Ok(call(state))
}
