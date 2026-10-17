//! The synthetic world benches 7 and 8 measure (SPR-009 T-01).
//!
//! A person ledger of [`COLUMNS`] `i64` columns, every column `rows` long, with values from a
//! keyed integer mix of (column, row): no RNG, no float, the same bytes on every machine, and
//! varied enough that zstd does real work. The engine has no systems: benches 7 and 8 measure the
//! hashes and keyframes of plane-A state, not a model, and the state's world columns are where its
//! size lives. Planes B and C have no tables yet (ADR-0024; SPR-007), so their hashes are measured
//! over synthetic byte rows of the same count, through the same Merkle code.
//!
//! DECISION: sizes are parameters; the PR tier runs [`PR_ROWS`] (SPR-009 ruling 2: 10k rows fit
//! the 15-minute budget under valgrind), the wall-clock trend 100k and 1M (`docs/08` §9; T-04).
//!
//! REF: `docs/08_performance_and_capacity.md` §9 benches 7 and 8; `CODEX/05_PROJECT/SPR-009_Benchmarks.md`
//! ruling 2.

use rh_core::crash::Provenance;
use rh_core::engine::Engine;
use rh_core::ownership::SpecTable;
use rh_core::register::{Registered, RegistrationError, register};
use rh_core::time::{Calendar, DaysPerYear, SeasonsPerYear, SimTime};
use rh_core::world::WorldTables;
use rh_ids::BranchSalt;
use rh_snapshot::journal::{JournalPosition, chain_genesis};
use rh_snapshot::keyframe::History;

/// Rows per table at the PR tier (SPR-009 ruling 2).
pub const PR_ROWS: usize = 10_000;

/// The ledger's columns, in name order.
pub const COLUMNS: [&str; 4] = [
    "person.alive",
    "person.birth",
    "person.household",
    "person.wealth",
];

/// The bytes of one synthetic plane-B or plane-C row (a fact or a sift match, in size).
pub const ROW_BYTES: usize = 24;

/// The world seed of the fixture engine.
pub const SEED: u64 = 9;

/// The calendar of the fixture engine.
pub const CALENDAR: Calendar = Calendar::new(DaysPerYear::D365, SeasonsPerYear::Four);

/// `SplitMix64`'s finaliser: a fixed integer mix, so the fixture is the same everywhere.
#[must_use]
pub const fn mix(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One cell: column `c`, row `r`, in a range that column could hold.
#[must_use]
pub fn cell(c: usize, r: usize) -> i64 {
    let bits = mix(((c as u64) << 40) ^ r as u64);
    let rows = r as u64 + 1;
    let value = match c {
        0 => u64::from(!bits.is_multiple_of(16)), // alive: 1 in 16 dead
        1 => bits % (200 * 365 * 24 * 60),        // birth: a minute in 200 years
        2 => bits % (rows / 5 + 1),               // household: about 5 persons each
        _ => bits % 1_000_000,                    // wealth: whole units
    };
    i64::try_from(value).unwrap_or(i64::MAX)
}

/// The ledger: every column of [`COLUMNS`], `rows` long.
#[must_use]
pub fn tables(rows: usize) -> WorldTables {
    let mut t = WorldTables::new();
    for (c, name) in COLUMNS.iter().enumerate() {
        t.insert(name, (0..rows).map(|r| cell(c, r)).collect());
    }
    t
}

/// Plane-B or plane-C tables: `(name, rows)`, each row its canonical bytes.
pub type ByteTables = Vec<(String, Vec<Vec<u8>>)>;

/// `tables` of plane-B or plane-C rows: one table per column name, each `rows` rows of
/// [`ROW_BYTES`] bytes.
#[must_use]
pub fn byte_tables(prefix: &str, rows: usize) -> ByteTables {
    COLUMNS
        .iter()
        .enumerate()
        .map(|(c, name)| {
            let rows = (0..rows)
                .map(|r| {
                    let mut row = Vec::with_capacity(ROW_BYTES);
                    for k in 0..ROW_BYTES / 8 {
                        row.extend_from_slice(
                            &mix((c * ROW_BYTES + k) as u64 ^ r as u64).to_le_bytes(),
                        );
                    }
                    row
                })
                .collect();
            (format!("{prefix}.{name}"), rows)
        })
        .collect()
}

/// The fixture's (empty) registered system set.
///
/// # Errors
/// Never in practice: an empty set has nothing to conflict.
pub fn registered() -> Result<Registered, RegistrationError> {
    register(Vec::new(), &SpecTable::default())
}

/// The history the fixture engine runs under.
#[must_use]
pub fn history() -> History {
    History {
        provenance: Provenance {
            build: "rh-bench".to_owned(),
            registry_version: "0.0.0".to_owned(),
            manifest_hash: "bench-ledger".to_owned(),
        },
        world_seed: SEED,
        branch_salt: 0,
    }
}

/// An engine over the ledger at `rows` rows.
///
/// # Errors
/// As [`registered`].
pub fn engine(rows: usize) -> Result<Engine, RegistrationError> {
    let mut e = Engine::new(
        CALENDAR,
        SEED,
        BranchSalt::from_raw(0),
        registered()?,
        tables(rows),
        SimTime::EPOCH,
    );
    e.set_provenance(history().provenance);
    Ok(e)
}

/// The journal position of an engine with no journal: no inputs, the genesis head.
#[must_use]
pub fn anchor() -> JournalPosition {
    JournalPosition {
        timeline_id: 0,
        inputs: 0,
        head: chain_genesis(0),
    }
}
