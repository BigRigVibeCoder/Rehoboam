//! `X5.smoke_polity` (`worlds/smoke/systems/X5.smoke_polity.odd.toml`): a yearly levy and a monthly
//! spend in the same phase, and a seasonal muster that sends `Settle` with no cause. The smoke
//! world's two bindings of one system in one phase, and its Polity and Military phases.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, CommandSend, Lag, Plane};
use rh_core::command::{Arg, Command};
use rh_core::fixed::Q16_16;
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};
use rh_events::{EventDraft, EventRef};

use crate::common::{check, fixed, from_skeleton, person, purposes, r64};

include!("generated/X5_smoke_polity.rs");
purposes!(MusterPick);

const TREASURY: &str = "polity.treasury";
const DECREES: &str = "polity.decrees";
const STOCK: &str = "harvest.stock";
const ALIVE: &str = "person.alive";

/// The polity system.
pub struct Polity;

impl Polity {
    fn levy(view: &mut SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let mut take = Q16_16::ZERO;
        for &s in view.read(STOCK)? {
            take = take.saturating_add(Q16_16::from_bits(fixed(SYSTEM_ID, s)?.to_bits() / 10));
        }
        let treasury = &mut view.write(TREASURY)?[0];
        *treasury = i64::from(fixed(SYSTEM_ID, *treasury)?.saturating_add(take).to_bits());
        view.write(DECREES)?[0] += 1;
        ctx.emit(EventDraft::new("DECREE").with_payload(vec![i64::from(take.to_bits())]))?;
        Ok(())
    }

    fn spend(view: &mut SystemView<'_>) -> Result<(), SimError> {
        let treasury = &mut view.write(TREASURY)?[0];
        let t = fixed(SYSTEM_ID, *treasury)?;
        *treasury = i64::from(
            t.saturating_sub(Q16_16::from_bits(t.to_bits() / 12))
                .to_bits(),
        );
        Ok(())
    }

    fn muster(view: &SystemView<'_>, ctx: &mut StepCtx<'_>) -> Result<(), SimError> {
        let alive = view.read(ALIVE)?.to_vec();
        for (r, &a) in alive.iter().enumerate() {
            if a == 1 && ctx.rng(&RngPurpose::MusterPick, person(r), 0)? % 16 == 0 {
                ctx.submit(Command {
                    kind: "Settle",
                    person: person(r),
                    args: vec![Arg::Int(r64(r))],
                    cause: EventRef::None,
                })?;
            }
        }
        Ok(())
    }
}

impl SystemModel for Polity {
    type Index = ();
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
                    lag: Lag::SameInstant,
                },
            ],
            writes: [TREASURY, DECREES]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
            sends: vec![CommandSend {
                command: "Settle",
                target: "X4.smoke_estate",
            }],
            emits: vec!["DECREE"],
            ..AccessSpec::default()
        }
    }

    fn step(
        &self,
        binding: PulseBinding,
        view: &mut SystemView<'_>,
        ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        match (binding.pulse, binding.phase) {
            (Pulse::Year, Phase::Polity) => Self::levy(view, ctx),
            (_, Phase::Polity) => Self::spend(view),
            _ => Self::muster(view, ctx),
        }
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let t = world
            .get(TREASURY)
            .and_then(|t| t.first())
            .copied()
            .unwrap_or(-1);
        vec![check("INV-X5-01", t >= 0, || {
            format!("the treasury {t} is negative")
        })]
    }
}
