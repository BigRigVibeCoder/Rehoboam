//! `rh-sys-regions`: the system `M1.regions` (`specs/systems/M1.regions.odd.toml`, 0.1.0 draft).
//!
//! Plane A, kind `system`. SPR-018 T-02 builds F0: the 18 owned columns, no binding, no step.
//! M1's tables are written once at genesis by `rh-worldgen` and never changed afterwards at F0.
//!
//! M1 F0 reads no column, binds no pulse, draws nothing of its own, and emits no event. Its only
//! input is the manifest, through `rh-worldgen`. Per X1 (SPR-018), INV-M1-01 to 05 are checked at
//! genesis and at restore, never in per-instant `invariants()`; `invariants()` is O(1).

#![forbid(unsafe_code)]

use rh_core::access::{AccessSpec, ColumnWrite, Plane};
use rh_core::system::{
    AggregateSpec, InvariantResult, ParamSchema, PulseBinding, SimError, StepCtx,
    SystemDescriptor, SystemModel,
};
use rh_core::world::{SystemView, WorldTables};

/// The owning system's Registry id string.
pub const SYSTEM_ID: &str = "M1.regions";

/// Spec version.
pub const SPEC_VERSION: &str = "0.1.0";

// ------------ owned columns (18, in spec order)
/// `raster.resolution`: the raster's side in cells.
pub const RASTER_RESOLUTION: &str = "raster.resolution";
/// `raster.sea_level`: the template's sea level.
pub const RASTER_SEA_LEVEL: &str = "raster.sea_level";
/// `cell.elevation`: elevation of each cell (u16, R*R rows).
pub const CELL_ELEVATION: &str = "cell.elevation";
/// `cell.region`: region id of each cell (0 = sea/unreached).
pub const CELL_REGION: &str = "cell.region";
/// `region.id`: region id, 1..K.
pub const REGION_ID: &str = "region.id";
/// `region.land_cells`: number of land cells in the region.
pub const REGION_LAND_CELLS: &str = "region.land_cells";
/// `region.seed_cell`: cell index of the region's seed.
pub const REGION_SEED_CELL: &str = "region.seed_cell";
/// `region.seat`: settlement id of the region's seat.
pub const REGION_SEAT: &str = "region.seat";
/// `settlement.id`: settlement id, 1..K at F0.
pub const SETTLEMENT_ID: &str = "settlement.id";
/// `settlement.region`: region the settlement lies in.
pub const SETTLEMENT_REGION: &str = "settlement.region";
/// `settlement.cell`: cell index of the settlement.
pub const SETTLEMENT_CELL: &str = "settlement.cell";
/// `region_edge.a`: lower region id of the edge.
pub const REGION_EDGE_A: &str = "region_edge.a";
/// `region_edge.b`: higher region id of the edge.
pub const REGION_EDGE_B: &str = "region_edge.b";
/// `region_edge.route`: route id along this edge.
pub const REGION_EDGE_ROUTE: &str = "region_edge.route";
/// `route.id`: route id, 1..m.
pub const ROUTE_ID: &str = "route.id";
/// `route.a`: settlement id of seat a.
pub const ROUTE_A: &str = "route.a";
/// `route.b`: settlement id of seat b.
pub const ROUTE_B: &str = "route.b";
/// `route.length_cells`: isqrt of squared distance between seats.
pub const ROUTE_LENGTH_CELLS: &str = "route.length_cells";

/// Every column M1 owns (`[[owns]]`), in spec order.
pub const OWNED: [&str; 18] = [
    RASTER_RESOLUTION, RASTER_SEA_LEVEL,
    CELL_ELEVATION, CELL_REGION,
    REGION_ID, REGION_LAND_CELLS, REGION_SEED_CELL, REGION_SEAT,
    SETTLEMENT_ID, SETTLEMENT_REGION, SETTLEMENT_CELL,
    REGION_EDGE_A, REGION_EDGE_B, REGION_EDGE_ROUTE,
    ROUTE_ID, ROUTE_A, ROUTE_B, ROUTE_LENGTH_CELLS,
];
// --------------------------------------------------------------------------- the system
/// M1.regions F0: the static region graph.
#[derive(Clone, Debug)]
pub struct M1Regions;

impl M1Regions {
    /// A new M1 system (no parameters at F0).
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for M1Regions {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemModel for M1Regions {
    type Index = ();

    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: SYSTEM_ID,
            fidelity: "F0",
            spec_version: SPEC_VERSION,
        }
    }

    fn access(&self) -> AccessSpec {
        AccessSpec {
            writes: OWNED
                .iter()
                .map(|&column| ColumnWrite { column, plane: Plane::A })
                .collect(),
            ..AccessSpec::default()
        }
    }

    fn schedule(&self) -> Vec<PulseBinding> {
        Vec::new()
    }

    fn rng_purposes(&self) -> Vec<&'static str> {
        Vec::new()
    }

    fn params_schema(&self) -> ParamSchema {
        ParamSchema {
            names: vec!["route_cost_sea", "route_cost_river", "route_cost_land"],
        }
    }

    fn step(
        &self,
        _binding: PulseBinding,
        _view: &mut SystemView<'_>,
        _ctx: &mut StepCtx<'_>,
    ) -> Result<(), SimError> {
        Ok(())
    }

    fn invariants(&self, tables: &WorldTables) -> Vec<InvariantResult> {
        let mut results = Vec::new();
        for col in &OWNED {
            if tables.get(col).is_none() {
                results.push(InvariantResult {
                    name: "INV-M1-06",
                    holds: false,
                    detail: format!("column {col} is missing from the tables"),
                });
            }
        }
        results
    }

    fn declared_aggregates(&self) -> Vec<AggregateSpec> {
        Vec::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_m1_regions() {
        let m1 = M1Regions::new();
        let d = m1.descriptor();
        assert_eq!(d.id, SYSTEM_ID);
        assert_eq!(d.fidelity, "F0");
    }

    #[test]
    fn owned_columns_count() {
        assert_eq!(OWNED.len(), 18, "M1 must own exactly 18 columns");
        let mut sorted = OWNED.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 18, "column names must be unique");
    }

    #[test]
    fn no_binding_registers() {
        let m1 = M1Regions::new();
        assert!(m1.schedule().is_empty(), "M1 F0 must bind no pulse");
    }

    #[test]
    fn invariants_are_o1() {
        let m1 = M1Regions::new();
        let tables = WorldTables::new();
        let results = m1.invariants(&tables);
        assert_eq!(results.len(), 18);
    }
}