//! `rh-worldgen`: world generation. In P1a, the founders stub of SPR-011 ruling 10 (SPR-012 T-06).
//!
//! Plane A, machinery, pinned in `bin/lint_arch.py`. [`genesis`] turns a compiled
//! [`WorldSpec`] into the initial tables: one ledger row and one life-course entry per founder,
//! in the columns `M2.demography` owns. It writes no raster, regions, settlements or households:
//! no spec owns them yet (P1.1a).
//!
//! **Order and ids.** Founders come in `WorldSpec` order (the cast, then each household's children
//! and servants, then the rest of the population; ruling 10), and founder `k` gets
//! `founder_base(seed, k)`, or its first free rehash if taken ([`founder_ids`]; ADR-0026 item 1).
//!
//! **Draws.** Under the reserved id `rh-worldgen` ([`rng_purposes`]), keyed by (founder, genesis):
//! - sex as given, else `founder_sex` k = 0: female iff the draw is below 2^63;
//! - age as given, else `founder_age` k = 0: uniform whole years in the role's range (children
//!   [0, 18), servants [15, 60), everyone else [0, 60); design values until P1.2);
//! - birth tick `−(age × Y) − offset`, `offset = floor(d · Y / 2^64)` from `founder_birthday` k = 0,
//!   with Y the calendar's minutes per year, so the founder's age at tick 0 is in [age, age + 1).
//!
//! **Ownership.** Genesis writes a column only if its owner's `[initialization] from_manifest`
//! names every manifest section a founder came from (`cast`, `households`, `peoples`); any other
//! write is refused with [`GenesisError::NotFromManifest`] (ruling 10).
//!
//! **Layout** (`specs/systems/M2.demography.odd.toml` `[initialization]`): ledger rows ascending by
//! `person.id`; `alive = 1`, `death_tick = i64::MAX`, `mother = father = spouse = 0`,
//! `death_cause = none`; life-course rows in ascending `PersonId`, `origin = founder`,
//! `entry_tick = 0`, exit fields open. No death timer exists at genesis.
//!
//! REF: `CODEX/05_PROJECT/SPR-011_WalkingSkeleton.md` ruling 10; `CODEX/05_PROJECT/SPR-012_WalkingSkeletonB.md`
//! T-06; ADR-0026; `05.1` §5, §6 item 2.
#![forbid(unsafe_code)]

pub mod rng_purposes;
pub mod worldgen_lite;

use core::fmt;

use rh_core::ownership::SpecTable;
use rh_core::register::system_id;
use rh_core::world::WorldTables;
use rh_ids::household::household_base;
use rh_ids::person::{IdError, PersonIdTable, founder_base};
use rh_ids::{BranchSalt, PersonId, PurposeId, SystemKey};
use rh_registry::{Founder, FounderRole, HouseholdSpec, Sex, WorldSpec};

pub use rng_purposes::RngPurpose;

/// The reserved RNG system id of world generation.
pub const WORLDGEN_ID: &str = "rh-worldgen";

/// `person.sex` and `life_course.sex` codes (M2 spec): male 0, female 1.
pub const SEX_MALE: i64 = 0;
/// See [`SEX_MALE`].
pub const SEX_FEMALE: i64 = 1;
/// `life_course.origin` of a founder (M2 spec: born 0, founder 1, immigrant 2).
pub const ORIGIN_FOUNDER: i64 = 1;
/// `marriage.source` of a manifest household's genesis marriage (H2; the M2 spec's
/// `[[owns]] marriage.source` values: genesis 0, partnering 1, command 2).
pub const MAR_SOURCE_GENESIS: i64 = 0;

/// Every column genesis writes, with its value per founder (the M2 spec's `[initialization]`).
const COLUMNS: [&str; 33] = [
    // person.* ledger columns (M2.demography owns them all).
    "person.id",
    "person.birth_tick",
    "person.sex",
    "person.alive",
    "person.death_tick",
    "person.death_cause",
    "person.mother",
    "person.father",
    "person.spouse",
    "person.household",
    "person.parity",
    "person.last_birth_tick",
    "person.conceived_tick",
    "person.unborn_father",
    // household.* table (SPR-015; initially empty at genesis).
    "household.id",
    "household.head",
    "household.formed_tick",
    "household.dissolved_tick",
    // marriage.* table (SPR-015; initially empty at genesis).
    "marriage.husband",
    "marriage.wife",
    "marriage.start_tick",
    "marriage.end_tick",
    "marriage.source",
    // life_course.* columns.
    "life_course.person_id",
    "life_course.origin",
    "life_course.mother",
    "life_course.father",
    "life_course.sex",
    "life_course.birth_tick",
    "life_course.entry_tick",
    "life_course.exit_kind",
    "life_course.exit_tick",
    "life_course.death_cause",
];

/// Why genesis refused a world.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GenesisError {
    /// A founder id could not be issued (the base and every rehash taken).
    Ids(IdError),
    /// A column genesis writes has no owner in the specs.
    NoOwner {
        /// The column.
        column: &'static str,
    },
    /// A column's owner does not initialise from a manifest section a founder came from.
    NotFromManifest {
        /// The column.
        column: &'static str,
        /// Its owner.
        owner: String,
        /// The section it does not name in `[initialization] from_manifest`.
        section: &'static str,
    },
    /// A birth tick that does not fit an `i64`.
    Overflow {
        /// The founder's index in genesis order.
        k: usize,
    },
}

impl fmt::Display for GenesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ids(e) => write!(f, "rh-worldgen: {e}"),
            Self::NoOwner { column } => write!(
                f,
                "rh-worldgen: {column} has no owner in the specs; genesis writes only owned columns"
            ),
            Self::NotFromManifest {
                column,
                owner,
                section,
            } => write!(
                f,
                "rh-worldgen: refused to write {column}: its owner {owner} does not initialise from \
                 the manifest section {section} ([initialization] from_manifest; SPR-011 ruling 10)"
            ),
            Self::Overflow { k } => write!(f, "rh-worldgen: founder {k}'s birth tick overflows"),
        }
    }
}

impl std::error::Error for GenesisError {}

/// The manifest section a founder of `role` comes from.
#[must_use]
pub const fn section(role: FounderRole) -> &'static str {
    match role {
        FounderRole::Cast => "cast",
        FounderRole::Child | FounderRole::Servant => "households",
        FounderRole::Commoner => "peoples",
    }
}

/// The ids of `count` founders of a world with `seed`, in genesis order: founder `k` gets
/// `founder_base(seed, k)` or its first free rehash, issued into `table` (pass a table that already
/// holds ids to force a collision; a new world passes an empty one).
///
/// # Errors
/// [`GenesisError::Ids`] when a base and all its rehashes are taken.
pub fn founder_ids(
    seed: u64,
    count: usize,
    table: &mut PersonIdTable,
) -> Result<Vec<PersonId>, GenesisError> {
    (0..count as u64)
        .map(|k| {
            table
                .issue(founder_base(seed, k))
                .map_err(GenesisError::Ids)
        })
        .collect()
}

/// The age range `[lo, hi)` in whole years for a founder of `role` with no age (ruling 10).
const fn age_range(role: FounderRole) -> (u64, u64) {
    match role {
        FounderRole::Child => (0, 18),
        FounderRole::Servant => (15, 60),
        FounderRole::Cast | FounderRole::Commoner => (0, 60),
    }
}

/// `floor(d · n / 2^64)`: a draw mapped uniformly onto `[0, n)`.
fn scaled(d: u64, n: u64) -> u64 {
    let v = (u128::from(d) * u128::from(n)) >> 64;
    // v < n.
    u64::try_from(v).unwrap_or(u64::MAX)
}

/// One founder as genesis lays it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FounderRow {
    /// The founder's id.
    pub id: PersonId,
    /// `person.sex` code.
    pub sex: i64,
    /// `person.birth_tick`, minutes (negative: before the epoch).
    pub birth_tick: i64,
    /// `person.household`: the founder's household id, never 0 at genesis (H2).
    pub household: i64,
    /// `person.mother` and `life_course.mother`.
    pub mother: i64,
    /// `person.father` and `life_course.father`.
    pub father: i64,
    /// `person.spouse`.
    pub spouse: i64,
    /// `person.parity`.
    pub parity: i64,
    /// `person.last_birth_tick`.
    pub last_birth_tick: i64,
}

/// Everything genesis lays out: the founder rows (in genesis order) and the household and marriage
/// table rows H2 derives from the manifest households.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Genesis {
    /// The founders, in genesis order (before the id sort [`write_columns`] applies).
    pub rows: Vec<FounderRow>,
    /// `household.*` rows as `[id, head, formed_tick, dissolved_tick]`, sorted by id.
    pub households: Vec<[i64; 4]>,
    /// `marriage.*` rows as `[husband, wife, start_tick, end_tick, source]`, in genesis order.
    pub marriages: Vec<[i64; 5]>,
}

/// The founders' rows and the household and marriage table rows, in genesis order.
///
/// # Errors
/// [`GenesisError::Ids`]; [`GenesisError::Overflow`].
pub fn founders(spec: &WorldSpec, table: &mut PersonIdTable) -> Result<Genesis, GenesisError> {
    let ids = founder_ids(spec.seed, spec.founders.len(), table)?;
    let key = SystemKey::new(
        spec.seed,
        BranchSalt::from_raw(0),
        system_id(WORLDGEN_ID),
        0,
    );
    let draw = |purpose: RngPurpose, id: PersonId| {
        key.stream_key(PurposeId::of(purpose.name()), id.to_raw())
            .draw(0, 0)
    };
    let year = spec.calendar.minutes_per_year();
    let mut rows: Vec<FounderRow> = ids
        .iter()
        .zip(&spec.founders)
        .enumerate()
        .map(|(k, (&id, &f))| row(k, id, f, year, &draw))
        .collect::<Result<_, _>>()?;
    let (households, marriages) = layout_households(&mut rows, &spec.households);
    Ok(Genesis {
        rows,
        households,
        marriages,
    })
}

/// Lays out H2: sets each founder's `person.household`, spouse, parents, parity and
/// `last_birth_tick`, and returns the `household.*` rows (sorted by id) and the `marriage.*` rows.
fn layout_households(
    rows: &mut [FounderRow],
    specs: &[HouseholdSpec],
) -> (Vec<[i64; 4]>, Vec<[i64; 5]>) {
    let mut households: Vec<[i64; 4]> = Vec::new();
    let mut marriages: Vec<[i64; 5]> = Vec::new();
    let mut covered = vec![false; rows.len()];

    for h in specs {
        let head = h.head;
        let head_id = rows[head].id.to_raw();
        let hh = household_base(head_id, 0).to_raw().cast_signed();

        // Membership: the head, the spouse, the children and the servants (H2).
        rows[head].household = hh;
        covered[head] = true;
        for k in h.children.clone() {
            rows[k].household = hh;
            covered[k] = true;
        }
        for k in h.servants.clone() {
            rows[k].household = hh;
            covered[k] = true;
        }

        // The married couple: the head and the one other adult of the other sex (H2). The wife is
        // the female, the husband the male.
        let mother = if rows[head].sex == SEX_FEMALE {
            Some(head)
        } else {
            h.spouse
        };
        let father = if rows[head].sex == SEX_MALE {
            Some(head)
        } else {
            h.spouse
        };
        if let Some(s) = h.spouse {
            rows[s].household = hh;
            covered[s] = true;
            rows[head].spouse = rows[s].id.to_raw().cast_signed();
            rows[s].spouse = head_id.cast_signed();
        }

        // The children's parents, in person.* and life_course.* (H2); 0 when the couple is not
        // married.
        for k in h.children.clone() {
            rows[k].mother = mother.map_or(0, |m| rows[m].id.to_raw().cast_signed());
            rows[k].father = father.map_or(0, |f| rows[f].id.to_raw().cast_signed());
        }

        // The wife's parity and last_birth_tick (REV-SPR-019 R14: one rule for every genesis
        // mother).
        let count = h.children.end - h.children.start;
        if let Some(m) = mother.filter(|_| count > 0) {
            rows[m].parity = i64::try_from(count).unwrap_or(i64::MAX);
            rows[m].last_birth_tick = h
                .children
                .clone()
                .map(|k| rows[k].birth_tick)
                .max()
                .unwrap_or(i64::MIN);
        }

        households.push([hh, head_id.cast_signed(), 0, i64::MAX]);
        if let Some(s) = h.spouse {
            let (husband, wife) = if rows[head].sex == SEX_MALE {
                (head, s)
            } else {
                (s, head)
            };
            marriages.push([
                rows[husband].id.to_raw().cast_signed(),
                rows[wife].id.to_raw().cast_signed(),
                0,
                i64::MAX,
                MAR_SOURCE_GENESIS,
            ]);
        }
    }

    // Every founder no manifest household covers gets a one-person household (H2).
    for (k, row) in rows.iter_mut().enumerate() {
        if covered[k] {
            continue;
        }
        let hh = household_base(row.id.to_raw(), 0).to_raw().cast_signed();
        row.household = hh;
        households.push([hh, row.id.to_raw().cast_signed(), 0, i64::MAX]);
    }

    // H1: the household table is sorted ascending by household.id.
    households.sort_by_key(|h| h[0].cast_unsigned());
    (households, marriages)
}

fn row(
    k: usize,
    id: PersonId,
    f: Founder,
    year: i64,
    draw: &impl Fn(RngPurpose, PersonId) -> u64,
) -> Result<FounderRow, GenesisError> {
    let female = match f.sex {
        Some(given) => given == Sex::Female,
        None => draw(RngPurpose::FounderSex, id) < 1 << 63,
    };
    let sex = if female { SEX_FEMALE } else { SEX_MALE };
    let age = f.age_years.map_or_else(
        || {
            let (lo, hi) = age_range(f.role);
            lo + scaled(draw(RngPurpose::FounderAge, id), hi - lo)
        },
        u64::from,
    );
    let offset = scaled(draw(RngPurpose::FounderBirthday, id), year.cast_unsigned());
    let birth_tick = i64::try_from(age)
        .ok()
        .and_then(|a| a.checked_mul(year))
        .and_then(|m| m.checked_add(offset.cast_signed()))
        .and_then(i64::checked_neg)
        .ok_or(GenesisError::Overflow { k })?;
    Ok(FounderRow {
        id,
        sex,
        birth_tick,
        household: 0,
        mother: 0,
        father: 0,
        spouse: 0,
        parity: 0,
        last_birth_tick: i64::MIN,
    })
}

/// Refuses a column whose owner does not initialise from every section in `sections`.
fn check_owner(
    specs: &SpecTable,
    column: &'static str,
    sections: &[&'static str],
) -> Result<(), GenesisError> {
    let owner = specs
        .owners
        .get(column)
        .ok_or(GenesisError::NoOwner { column })?;
    let from = specs
        .specs
        .get(&owner.system)
        .map(|e| e.from_manifest.as_slice())
        .unwrap_or_default();
    match sections.iter().find(|s| !from.iter().any(|f| f == *s)) {
        Some(&section) => Err(GenesisError::NotFromManifest {
            column,
            owner: owner.system.clone(),
            section,
        }),
        None => Ok(()),
    }
}

/// The initial tables of `spec`: the founders in M2's ledger and life-course columns (module
/// docs). `specs` is the spec table registration uses; it says who owns each column and which
/// manifest sections the owner initialises from.
///
/// # Errors
/// [`GenesisError`]: an id that cannot be issued, a column without an owner or whose owner does
/// not initialise from the manifest, or an overflowing birth tick.
pub fn genesis(spec: &WorldSpec, specs: &SpecTable) -> Result<WorldTables, GenesisError> {
    genesis_with(spec, specs, &mut PersonIdTable::new())
}

/// [`genesis`] issuing ids into `table` (a test forces a collision by pre-filling it).
///
/// # Errors
/// As [`genesis`].
pub fn genesis_with(
    spec: &WorldSpec,
    specs: &SpecTable,
    table: &mut PersonIdTable,
) -> Result<WorldTables, GenesisError> {
    let mut sections: Vec<&'static str> = spec.founders.iter().map(|f| section(f.role)).collect();
    sections.sort_unstable();
    sections.dedup();
    write_columns(specs, &sections, founders(spec, table)?)
}

/// Writes the founders' rows and the household and marriage table rows into the columns, after
/// checking each column's owner.
///
/// # Errors
/// [`GenesisError::NoOwner`]; [`GenesisError::NotFromManifest`].
pub fn write_columns(
    specs: &SpecTable,
    sections: &[&'static str],
    genesis: Genesis,
) -> Result<WorldTables, GenesisError> {
    for column in COLUMNS {
        check_owner(specs, column, sections)?;
    }
    let Genesis {
        mut rows,
        households,
        marriages,
    } = genesis;
    rows.sort_by_key(|r| r.id);
    let mut tables = WorldTables::new();
    let n = rows.len();
    let ids: Vec<i64> = rows.iter().map(|r| r.id.to_raw().cast_signed()).collect();
    let sexes: Vec<i64> = rows.iter().map(|r| r.sex).collect();
    let births: Vec<i64> = rows.iter().map(|r| r.birth_tick).collect();
    let memberships: Vec<i64> = rows.iter().map(|r| r.household).collect();
    let mothers: Vec<i64> = rows.iter().map(|r| r.mother).collect();
    let fathers: Vec<i64> = rows.iter().map(|r| r.father).collect();
    let spouses: Vec<i64> = rows.iter().map(|r| r.spouse).collect();
    let parities: Vec<i64> = rows.iter().map(|r| r.parity).collect();
    let last_births: Vec<i64> = rows.iter().map(|r| r.last_birth_tick).collect();
    let hh_ids: Vec<i64> = households.iter().map(|h| h[0]).collect();
    let hh_heads: Vec<i64> = households.iter().map(|h| h[1]).collect();
    let hh_formed: Vec<i64> = households.iter().map(|h| h[2]).collect();
    let hh_dissolved: Vec<i64> = households.iter().map(|h| h[3]).collect();
    let mar_husbands: Vec<i64> = marriages.iter().map(|m| m[0]).collect();
    let mar_wives: Vec<i64> = marriages.iter().map(|m| m[1]).collect();
    let mar_starts: Vec<i64> = marriages.iter().map(|m| m[2]).collect();
    let mar_ends: Vec<i64> = marriages.iter().map(|m| m[3]).collect();
    let mar_sources: Vec<i64> = marriages.iter().map(|m| m[4]).collect();
    for column in COLUMNS {
        let values = match column {
            "person.id" | "life_course.person_id" => ids.clone(),
            "person.sex" | "life_course.sex" => sexes.clone(),
            "person.birth_tick" | "life_course.birth_tick" => births.clone(),
            "person.alive" => vec![1; n],
            "person.death_tick" | "life_course.exit_tick" => vec![i64::MAX; n],
            "person.conceived_tick" => vec![i64::MIN; n],
            "person.last_birth_tick" => last_births.clone(),
            "person.household" => memberships.clone(),
            "person.mother" | "life_course.mother" => mothers.clone(),
            "person.father" | "life_course.father" => fathers.clone(),
            "person.spouse" => spouses.clone(),
            "person.parity" => parities.clone(),
            "person.death_cause" | "person.unborn_father" => vec![0; n],
            "life_course.origin" => vec![ORIGIN_FOUNDER; n],
            "life_course.entry_tick" | "life_course.exit_kind" | "life_course.death_cause" => {
                vec![0; n]
            }
            "household.id" => hh_ids.clone(),
            "household.head" => hh_heads.clone(),
            "household.formed_tick" => hh_formed.clone(),
            "household.dissolved_tick" => hh_dissolved.clone(),
            "marriage.husband" => mar_husbands.clone(),
            "marriage.wife" => mar_wives.clone(),
            "marriage.start_tick" => mar_starts.clone(),
            "marriage.end_tick" => mar_ends.clone(),
            "marriage.source" => mar_sources.clone(),
            _ => unreachable!("unhandled genesis column {column}"),
        };
        tables.insert(column, values);
    }
    Ok(tables)
}
