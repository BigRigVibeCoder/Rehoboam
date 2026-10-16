//! `X7.smoke_market` (`worlds/smoke/systems/X7.smoke_market.odd.toml`): a Q16.16 price per region
//! that moves every day beside the weather, and a daily trade beside the estates that applies the
//! `Audit` commands the estates send it and sends `Settle` for a few living persons.
//!
//! Its reason to exist is VER-SPR-008 DEF-2: it shares a scheduler level with `X1.smoke_weather`
//! (Environment) and with `X4.smoke_estate` (Activity) every day, reading nothing they write in
//! those phases, so those levels hold two jobs and the thread-count comparison dispatches them in
//! parallel. Daily, not hourly: the engine spawns scoped threads for every level that holds two
//! jobs, and hourly parallel levels made a 1,000-year run at 4 threads take about 25 minutes
//! (100 years: 150 s hourly, 23 s daily). Both of its steps draw, write and emit; the Activity
//! one also applies and sends lifecycle commands, so a dispatch or merge bug changes the hash.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandSend, Lag, Plane};
use rh_core::command::Command;
use rh_core::fixed::Q16_16;
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventDraft;
use rh_ids::EntityRef;

use crate::common::{
    REGIONS, check, degrees, fail, fixed, from_skeleton, person, purposes, r64, region,
};

include!("generated/X7_smoke_market.rs");
purposes!(PriceStep, DefaultPick);

const PRICE: &str = "market.price";
const TICKS: &str = "market.ticks";
const AUDITS: &str = "market.audits";
const ALIVE: &str = "person.alive";
const STOCK: &str = "harvest.stock";
const MIN: i16 = 1;
const MAX: i16 = 1_000;

/// The market system.
pub struct Market;

impl Market {
    /// Every day: each region's price takes a keyed step, pulled toward 100.
    fn tick(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let (lo, hi, mean) = (
            Q16_16::from_int(MIN),
            Q16_16::from_int(MAX),
            Q16_16::from_int(100),
        );
        let mut large = Vec::new();
        for (r, bits) in view.write(PRICE)?.iter_mut().enumerate().take(REGIONS) {
            let p = fixed(SYSTEM_ID, *bits)?;
            let u = ctx.rng(&RngPurpose::PriceStep, region(r), 0)?;
            let k = i32::try_from(u % 33).unwrap_or(0) - 16;
            let step = Q16_16::from_bits(k * 1_024);
            let pull = Q16_16::from_bits(p.saturating_sub(mean).to_bits() / 128);
            *bits = i64::from(
                p.saturating_add(step)
                    .saturating_sub(pull)
                    .max(lo)
                    .min(hi)
                    .to_bits(),
            );
            if k.abs() >= 15 {
                large.push((r, k));
            }
        }
        for (r, k) in large {
            ctx.emit(
                EventDraft::new("TRADE")
                    .subject(EntityRef::region(region(r)))
                    .with_payload(vec![r64(r), i64::from(k)]),
            )?;
        }
        view.write(TICKS)?[0] += 1;
        Ok(())
    }

    /// Every day: apply the audits, trade, and send a few living persons' estates to be settled.
    fn trade(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let audits = ctx.take_commands();
        if !audits.windows(2).all(|w| w[0].key() < w[1].key()) {
            return Err(fail(
                SYSTEM_ID,
                "the Audit queue is not in (tick, phase, PersonId, seq) order".to_owned(),
            ));
        }
        let applied = i64::try_from(audits.len()).unwrap_or(i64::MAX);
        view.write(AUDITS)?[0] += applied;
        let stock: i64 = view.read(STOCK)?.iter().sum::<i64>() / degrees(1).max(1);
        let alive = view.read(ALIVE)?.to_vec();
        let mut defaults = 0;
        for (r, &a) in alive.iter().enumerate() {
            if a == 1 && ctx.rng(&RngPurpose::DefaultPick, person(r), 0)? % 256 == 0 {
                ctx.submit(Command {
                    kind: "Settle",
                    person: person(r),
                    args: vec![r64(r)],
                    cause: None,
                })?;
                defaults += 1;
            }
        }
        ctx.emit(EventDraft::new("TRADE").with_payload(vec![applied, defaults, stock]))?;
        Ok(())
    }
}

impl SystemModel for Market {
    from_skeleton!();

    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![
                ColumnRead {
                    column: STOCK,
                    owner: "X2.smoke_harvest",
                    lag: Lag::Lagged,
                },
                ColumnRead {
                    column: ALIVE,
                    owner: "X3.smoke_vital",
                    lag: Lag::Lagged,
                },
            ],
            writes: [PRICE, TICKS, AUDITS]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            sends: vec![CommandSend {
                command: "Settle",
                target: "X4.smoke_estate",
            }],
            applies: vec!["Audit"],
            emits: vec!["TRADE"],
        }
    }

    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        match (binding.pulse, binding.phase) {
            (_, Phase::Environment) => Self::tick(view, ctx),
            _ => Self::trade(view, ctx),
        }
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let prices = world.get(PRICE).map_or(&[][..], Vec::as_slice);
        let audits = world
            .get(AUDITS)
            .and_then(|a| a.first())
            .copied()
            .unwrap_or(-1);
        vec![
            check(
                "INV-X7-01",
                prices.len() == REGIONS
                    && prices
                        .iter()
                        .all(|&p| (degrees(MIN)..=degrees(MAX)).contains(&p)),
                || format!("prices out of [{MIN}, {MAX}]: {prices:?}"),
            ),
            check("INV-X7-02", audits >= 0, || {
                format!("the audit count {audits} is negative")
            }),
        ]
    }
}
