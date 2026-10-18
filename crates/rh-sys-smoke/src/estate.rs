//! `X4.smoke_estate` (`worlds/smoke/systems/X4.smoke_estate.odd.toml`): owns and applies
//! `Settle` in `(tick, phase, PersonId, seq)` order, emitting `INHERITANCE` caused by the death
//! that sent it; counts gossip days. The smoke world's lifecycle-command owner and its Activity and
//! Contagion phases.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandApply, CommandSend, Lag, Plane};
use rh_core::command::{Arg, Command};
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EdgeKind, EventDraft, EventRef};
use rh_ids::EntityRef;

use crate::common::{PERSONS, WORLD, check, fail, from_skeleton, purposes, row};

include!("generated/X4_smoke_estate.rs");
purposes!(GossipNoise, SettleFee);

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
            let r = row(SYSTEM_ID, q.command.int(0).unwrap_or(-1), PERSONS)?;
            // DEF-2: the Activity siblings both draw, write, emit and send commands.
            let fee = ctx.rng(&RngPurpose::SettleFee, q.command.person, q.seq)?;
            view.write(HEIRS)?[r] += 1 + i64::from(fee % 2 == 1);
            let order = &mut view.write(ORDER)?[0];
            *order = order
                .wrapping_mul(31)
                .wrapping_add(q.command.person.to_raw().cast_signed());
            let mut e = EventDraft::new("INHERITANCE")
                .role(ROLE_TARGET, EntityRef::person(q.command.person));
            if let Some(cause) = q.cause {
                e = e.caused_by(EventRef::Known(cause), EdgeKind::Triggered);
            }
            let inheritance = ctx.emit(e)?;
            if (fee >> 8) % 4 == 0 {
                ctx.submit(Command {
                    kind: "Audit",
                    person: q.command.person,
                    args: vec![Arg::Int(crate::common::r64(r))],
                    cause: EventRef::Draft(inheritance),
                })?;
            }
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
            applies: CommandApply::from_drains(DRAINS),
            sends: vec![CommandSend {
                command: "Audit",
                target: "X7.smoke_market",
            }],
            emits: vec!["INHERITANCE"],
            drain_only: Vec::new(),
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
