//! `X4.smoke_estate` (`worlds/smoke/systems/X4.smoke_estate.odd.toml`): owns and applies
//! `Settle` in `(tick, phase, PersonId, seq)` order, emitting `INHERITANCE` caused by the death
//! that sent it; counts gossip days. The smoke world's lifecycle-command owner and its Activity and
//! Contagion phases.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::EventDraft;
use rh_ids::EntityRef;

use crate::common::{PERSONS, WORLD, check, fail, from_skeleton, purposes, row};

include!("generated/X4_smoke_estate.rs");
purposes!(GossipNoise);

const HEIRS: &str = "estate.heirs";
const ORDER: &str = "estate.order";
const GOSSIP: &str = "estate.gossip";
const ALIVE: &str = "person.alive";

/// The estate system.
pub struct Estate;

impl Estate {
    fn settle(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let queue = ctx.take_commands();
        if !queue.windows(2).all(|w| w[0].key() < w[1].key()) {
            return Err(fail(
                SYSTEM_ID,
                "the Settle queue is not in (tick, phase, PersonId, seq) order".to_owned(),
            ));
        }
        for q in queue {
            let r = row(
                SYSTEM_ID,
                q.command.args.first().copied().unwrap_or(-1),
                PERSONS,
            )?;
            view.write(HEIRS)?[r] += 1;
            let order = &mut view.write(ORDER)?[0];
            *order = order
                .wrapping_mul(31)
                .wrapping_add(q.command.person.to_raw().cast_signed());
            let mut e = EventDraft::new("INHERITANCE").subject(EntityRef::person(q.command.person));
            if let Some(cause) = q.cause {
                e = e.caused_by(cause);
            }
            ctx.emit(e)?;
        }
        Ok(())
    }

    fn gossip(view: &mut SystemView<'_>, ctx: &StepCtx<'_>) -> Result<(), SimError> {
        let living: i64 = view.read(ALIVE)?.iter().sum();
        let coin = i64::from(ctx.rng(&RngPurpose::GossipNoise, WORLD, 0)? % 2 == 1);
        view.write(GOSSIP)?[0] += (living + coin) % 2;
        Ok(())
    }
}

impl SystemModel for Estate {
    from_skeleton!();

    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![ColumnRead {
                column: ALIVE,
                owner: "X3.smoke_vital",
                lag: Lag::SameInstant,
            }],
            writes: [HEIRS, ORDER, GOSSIP]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            applies: vec!["Settle"],
            emits: vec!["INHERITANCE"],
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
            Phase::Activity => Self::settle(view, ctx),
            _ => Self::gossip(view, ctx),
        }
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let heirs = world.get(HEIRS).map_or(&[][..], Vec::as_slice);
        let gossip = world
            .get(GOSSIP)
            .and_then(|g| g.first())
            .copied()
            .unwrap_or(-1);
        vec![
            check(
                "INV-X4-01",
                heirs.len() == PERSONS && heirs.iter().all(|&h| h >= 0),
                || format!("heir counts must be non-negative: {heirs:?}"),
            ),
            check("INV-X4-02", gossip >= 0, || {
                format!("the gossip count {gossip} is negative")
            }),
        ]
    }
}
