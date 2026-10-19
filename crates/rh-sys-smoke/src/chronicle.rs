//! `X6.smoke_chronicle` (`worlds/smoke/systems/X6.smoke_chronicle.odd.toml`): a monthly census
//! with a note caused by it (a draft-local causal edge), and the days counted in Culture and closed
//! in Bookkeeping. The smoke world's Agenda, Culture and Bookkeeping phases.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventDraft;

use crate::common::{PERSONS, WORLD, check, from_skeleton, purposes, r64};

include!("generated/X6_smoke_chronicle.rs");
purposes!(NoteTone);

const CENSUS: &str = "chronicle.census";
const DAYS: &str = "chronicle.days";
const CLOSED: &str = "chronicle.closed";
const ALIVE: &str = "person.alive";
const HEIRS: &str = "estate.heirs";

/// The chronicle system.
pub struct Chronicle;

impl Chronicle {
    fn census(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let living: i64 = view.read(ALIVE)?.iter().sum();
        let heirs: i64 = view.read(HEIRS)?.iter().sum();
        view.write(CENSUS)?[0] = living;
        let census = ctx.emit(EventDraft::new("CENSUS").with_payload(vec![living, heirs]))?;
        let tone = ctx.rng(&RngPurpose::NoteTone, WORLD, 0)? % 4;
        ctx.emit(
            EventDraft::new("CENSUS_NOTE")
                .after(census)
                .with_payload(vec![tone.cast_signed()]),
        )?;
        Ok(())
    }
}

impl SystemModel for Chronicle {
    type Index = ();
    from_skeleton!();

    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![
                ColumnRead {
                    column: ALIVE,
                    owner: "X3.smoke_vital",
                    lag: Lag::SameInstant,
                },
                ColumnRead {
                    column: HEIRS,
                    owner: "X4.smoke_estate",
                    lag: Lag::Lagged,
                },
            ],
            writes: [CENSUS, DAYS, CLOSED]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            emits: vec!["CENSUS", "CENSUS_NOTE"],
            ..AccessSpec::default()
        }
    }

    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        match binding.phase {
            Phase::Agenda => Self::census(view, ctx),
            Phase::Culture => {
                view.write(DAYS)?[0] += 1;
                Ok(())
            }
            _ => {
                view.write(CLOSED)?[0] += 1;
                Ok(())
            }
        }
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let first = |c: &str| world.get(c).and_then(|v| v.first()).copied().unwrap_or(-1);
        let (census, days, closed) = (first(CENSUS), first(DAYS), first(CLOSED));
        vec![
            check("INV-X6-01", (0..=r64(PERSONS)).contains(&census), || {
                format!("the census {census} is outside [0, {PERSONS}]")
            }),
            check("INV-X6-02", days == closed, || {
                format!("{days} days counted but {closed} closed")
            }),
        ]
    }
}
