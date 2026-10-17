// Negative control for bin/lint_forbidden.py (V-DET-19 float rule, SPR-010 T-04): a float in a declared
// float field, escaped with `f32_field <entity.column>` naming a column that specs/X1.fields.odd.toml
// declares f32_field. Scanned with `--specs tests/lint_fixtures/forbidden/float_field/specs`, it must
// PASS with one counted escape. Never compiled.

/// A raster cell: soil moisture is a continuous field, not a threshold.
pub struct Cell {
    pub soil_moisture: f32, // LINT-ALLOW(forbidden-api): f32_field region.soil_moisture
}
