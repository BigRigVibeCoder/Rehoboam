//! `X2.smoke_harvest` (`worlds/smoke/systems/X2.smoke_harvest.odd.toml`): a Q16.16 grain stock
//! per region that grows each season from a `lagged` read of the weather and is evened between
//! regions each month. The smoke world's Season pulse and its Production and Exchange phases.
#![allow(
    dead_code,
    reason = "generated skeleton items (Params) a parameterless system does not use"
)]

use rh_core::access::{AccessSpec, ColumnRead, ColumnWrite, Lag, Plane};
use rh_core::fixed::Q16_16;
use rh_core::system::{InvariantResult, PulseBinding, SimError, StepCtx, SystemModel};
use rh_core::world::{SystemView, WorldTables};
use rh_core::{Phase, Pulse};

use crate::common::{REGIONS, check, degrees, fixed, from_skeleton, purposes, region};

include!("generated/X2_smoke_harvest.rs");
purposes!(YieldNoise);

const STOCK: &str = "harvest.stock";
const LAST_YIELD: &str = "harvest.last_yield";
const TEMP: &str = "weather.temp";
const CAP: i16 = 10_000;

/// The harvest system.
pub struct Harvest;

impl Harvest {
    fn grow(view: &mut SystemView<'_>, ctx: &StepCtx<'_>) -> Result<(), SimError> {
        let temps: Vec<Q16_16> = view
            .read(TEMP)?
            .iter()
            .map(|&b| fixed(SYSTEM_ID, b))
            .collect::<Result<_, _>>()?;
        let (zero, thirty) = (Q16_16::from_int(0), Q16_16::from_int(30));
        let cap = Q16_16::from_int(CAP);
        let mut yields = Vec::with_capacity(REGIONS);
        for (r, &t) in temps.iter().enumerate().take(REGIONS) {
            let warmth = Q16_16::from_bits(t.max(zero).min(thirty).to_bits() / 2);
            let u = ctx.rng(&RngPurpose::YieldNoise, region(r), 0)?;
            let noise = Q16_16::from_int(i16::try_from(u % 11).unwrap_or(0) - 5);
            yields.push(
                Q16_16::from_int(20)
                    .saturating_add(warmth)
                    .saturating_add(noise),
            );
        }
        let stock = view.write(STOCK)?;
        for (bits, y) in stock.iter_mut().zip(&yields) {
            let s = fixed(SYSTEM_ID, *bits)?;
            let kept = s.saturating_sub(Q16_16::from_bits(s.to_bits() / 8));
            *bits = i64::from(kept.saturating_add(*y).max(Q16_16::ZERO).min(cap).to_bits());
        }
        for (bits, y) in view.write(LAST_YIELD)?.iter_mut().zip(&yields) {
            *bits = i64::from(y.to_bits());
        }
        Ok(())
    }

    /// Each region above the mean gives an eighth of its surplus to the region with the least
    /// stock (the first such region in row order).
    fn trade(view: &mut SystemView<'_>) -> Result<(), SimError> {
        let stock = view.write(STOCK)?;
        let mean = stock.iter().sum::<i64>() / i64::try_from(stock.len().max(1)).unwrap_or(1);
        for giver in 0..stock.len() {
            let poorest = (0..stock.len()).min_by_key(|&r| (stock[r], r)).unwrap_or(0);
            let surplus = stock[giver] - mean;
            if surplus > 0 && poorest != giver {
                stock[giver] -= surplus / 8;
                stock[poorest] += surplus / 8;
            }
        }
        Ok(())
    }
}

impl SystemModel for Harvest {
    from_skeleton!();

    fn access(&self) -> AccessSpec {
        AccessSpec {
            reads: vec![ColumnRead {
                column: TEMP,
                owner: "X1.smoke_weather",
                lag: Lag::Lagged,
            }],
            writes: [STOCK, LAST_YIELD]
                .map(|column| ColumnWrite {
                    column,
                    plane: Plane::A,
                })
                .to_vec(),
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
            (Pulse::Season, Phase::Production) => Self::grow(view, ctx),
            _ => Self::trade(view),
        }
    }

    fn invariants(&self, world: &WorldTables) -> Vec<InvariantResult> {
        let stock = world.get(STOCK).map_or(&[][..], Vec::as_slice);
        vec![check(
            "INV-X2-01",
            stock.len() == REGIONS && stock.iter().all(|&s| (0..=degrees(CAP)).contains(&s)),
            || format!("stocks out of [0, {CAP}]: {stock:?}"),
        )]
    }
}
