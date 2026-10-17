//! `X3.smoke_vital` (`worlds/smoke/systems/X3.smoke_vital.odd.toml`): death timers, cures, births
//! and `Settle` commands. The smoke world's timers (scheduled, fired, cancelled, re-sampled by a
//! material input; ADR-0025) and its Vital phase.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandSend, Lag, Plane};
use rh_core::agenda::TimerEntry;
use rh_core::command::Command;
use rh_core::system::{
    InvariantResult, PulseBinding, ResampleCtx, SimError, StepCtx, SystemModel, TimerSpec,
};
use rh_core::time::SimTime;
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EventDraft, RoleId};

use crate::common::{
    PERSONS, check, degrees, fail, from_skeleton, person, person_entity, purposes, r64, row,
};

include!("generated/X3_smoke_vital.rs");
purposes!(DeathTime, Cure, Birth);

const ALIVE: &str = "person.alive";
const PENDING: &str = "person.pending";
const DEATHS: &str = "person.deaths";
const STOCK: &str = "harvest.stock";
/// Death timers land 1 to this many minutes ahead (60 days), so some outlive the month.
const HORIZON: u64 = 60 * 1_440;
/// Below this stock in any region, deaths come faster.
const FAMINE_STOCK: i16 = 20;

/// The vital system.
pub struct Vital;

impl Vital {
    fn sample(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let famine = view.read(STOCK)?.iter().any(|&s| s < degrees(FAMINE_STOCK));
        let odds = if famine { 6 } else { 12 };
        let mut alive = view.read(ALIVE)?.to_vec();
        let mut pending = view.read(PENDING)?.to_vec();
        for r in 0..PERSONS {
            let p = person(r);
            if alive[r] == 0 {
                if ctx.rng(&RngPurpose::Birth, p, 0)? % 3 == 0 {
                    alive[r] = 1;
                    ctx.emit(EventDraft::new("BIRTH").role(RoleId::TARGET, person_entity(r)))?;
                }
            } else if pending[r] == 0 {
                if ctx.rng(&RngPurpose::DeathTime, p, 0)? % odds == 0 {
                    let offset = 1 + ctx.rng(&RngPurpose::DeathTime, p, 1)? % HORIZON;
                    let time = ctx
                        .now()
                        .checked_add_minutes(offset.cast_signed())
                        .ok_or_else(|| fail(SYSTEM_ID, "a death time overflows".to_owned()))?;
                    ctx.schedule(
                        &RngPurpose::DeathTime,
                        TimerSpec {
                            time,
                            phase: Phase::Vital,
                            entity: person_entity(r),
                            kind: 0,
                            payload: r64(r),
                        },
                    )?;
                    pending[r] = 1;
                }
            } else if ctx.rng(&RngPurpose::Cure, p, 0)? % 5 == 0 {
                for t in ctx.own_timers(person_entity(r)) {
                    ctx.cancel(t.id())?;
                }
                pending[r] = 0;
                ctx.emit(EventDraft::new("CURED").role(RoleId::TARGET, person_entity(r)))?;
            }
        }
        view.write(ALIVE)?.copy_from_slice(&alive);
        view.write(PENDING)?.copy_from_slice(&pending);
        Ok(())
    }
}

impl SystemModel for Vital {
    from_skeleton!();

    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![ColumnRead {
                column: STOCK,
                owner: "X2.smoke_harvest",
                lag: Lag::SameInstant,
            }],
            writes: [ALIVE, PENDING, DEATHS]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            sends: vec![CommandSend {
                command: "Settle",
                target: "X4.smoke_estate",
            }],
            emits: vec!["X_DEATH", "CURED", "BIRTH"],
            ..AccessSpec::default()
        }
    }

    fn step(
        &self,
        _: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Self::sample(view, ctx)
    }

    /// A death: the person dies, `X_DEATH` is emitted, and `Settle` goes to the estate with the
    /// death as its cause. A timer for a person with no pending death is a bug (a cancelled
    /// timer that fired) and stops the run.
    fn fire(
        &self,
        timer: &TimerEntry,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        let r = row(SYSTEM_ID, timer.timer.payload, PERSONS)?;
        if view.read(PENDING)?[r] != 1 || view.read(ALIVE)?[r] != 1 {
            return Err(fail(
                SYSTEM_ID,
                format!(
                    "{} fired for row {r}, which has no pending death",
                    timer.id()
                ),
            ));
        }
        view.write(ALIVE)?[r] = 0;
        view.write(PENDING)?[r] = 0;
        view.write(DEATHS)?[0] += 1;
        let death = ctx.emit(
            EventDraft::new("X_DEATH")
                .role(RoleId::TARGET, person_entity(r))
                .with_payload(vec![r64(r), timer.key.time.minutes()]),
        )?;
        ctx.submit(Command {
            kind: "Settle",
            person: person(r),
            args: vec![r64(r)],
            cause: Some(death),
        })
    }

    /// A material input re-draws the death time from its epoch-keyed stream (ADR-0025 item 3).
    fn resample_timer(
        &self,
        _: &TimerEntry,
        ctx: &ResampleCtx,
    ) -> Result<Option<SimTime>, SimError> {
        let offset = 1 + ctx.draw(0) % HORIZON;
        Ok(ctx.now().checked_add_minutes(offset.cast_signed()))
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let alive = world.get(ALIVE).map_or(&[][..], Vec::as_slice);
        let pending = world.get(PENDING).map_or(&[][..], Vec::as_slice);
        let orphan = alive
            .iter()
            .zip(pending)
            .position(|(&a, &p)| p == 1 && a != 1);
        vec![
            check("INV-X3-01", orphan.is_none(), || {
                format!("row {orphan:?} has a pending death but is not alive")
            }),
            check(
                "INV-X3-02",
                alive.len() == PERSONS
                    && pending.len() == PERSONS
                    && alive.iter().chain(pending).all(|&v| v == 0 || v == 1),
                || "alive and pending must be 0 or 1 on every person row".to_owned(),
            ),
        ]
    }
}
