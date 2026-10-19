//! Reference differential test for terrain_v0 voxelization (SPR-033 T-03).
//!
//! Verifies that Rust `TerrainContext::eval_column` reproduces all 400 golden columns
//! from `specs/viewer/golden/terrain_v0_columns.txt` line for line.
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use std::fs;
use std::path::Path;

use rh_view_core::export::WorldgenExport;
use rh_view_core::terrain::TerrainContext;

#[test]
fn test_terrain_differential_400_columns() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let golden_path = root.join("specs/viewer/golden/terrain_v0_columns.txt");
    let golden_content = fs::read_to_string(&golden_path).unwrap_or_else(|e| {
        panic!(
            "failed to read golden columns {}: {e}",
            golden_path.display()
        )
    });

    let export_dir = root.join("target/viewer/exports");
    let export_toml = export_dir.join("hamlet.toml");
    assert!(
        export_toml.exists(),
        "hamlet export not found at {}: run export_worldgen_lite.py --world hamlet first",
        export_toml.display()
    );

    let export = WorldgenExport::load_from_dir(&export_dir, "hamlet")
        .unwrap_or_else(|e| panic!("failed to load hamlet export: {e}"));
    let ctx = TerrainContext::new(&export);

    for (line_idx, line) in golden_content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Expected format:
        // col i=0 vx=9387 vz=9872 e=23528 h=1050 surface=3 stack=3,4,4,4,5,5 flag=none
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        assert!(parts.len() >= 8, "invalid column line format: {line}");

        let i_val: usize = parts[1].strip_prefix("i=").unwrap().parse().unwrap();
        assert_eq!(i_val, line_idx);
        let vx: i64 = parts[2].strip_prefix("vx=").unwrap().parse().unwrap();
        let vz: i64 = parts[3].strip_prefix("vz=").unwrap().parse().unwrap();
        let expected_e: i64 = parts[4].strip_prefix("e=").unwrap().parse().unwrap();
        let expected_h: i64 = parts[5].strip_prefix("h=").unwrap().parse().unwrap();
        let expected_surf: u16 = parts[6].strip_prefix("surface=").unwrap().parse().unwrap();
        let stack_str = parts[7].strip_prefix("stack=").unwrap();
        let expected_stack: Vec<u16> = stack_str.split(',').map(|s| s.parse().unwrap()).collect();
        assert_eq!(expected_stack.len(), 6);
        let expected_flag = parts[8].strip_prefix("flag=").unwrap();

        let col = ctx.eval_column(vx, vz);

        assert_eq!(
            col.e, expected_e,
            "col {i_val} (vx={vx}, vz={vz}): e mismatch"
        );
        assert_eq!(
            col.h, expected_h,
            "col {i_val} (vx={vx}, vz={vz}): h mismatch"
        );
        assert_eq!(
            col.surface, expected_surf,
            "col {i_val} (vx={vx}, vz={vz}): surface mismatch"
        );
        assert_eq!(
            &col.stack[..],
            &expected_stack[..],
            "col {i_val} (vx={vx}, vz={vz}): stack mismatch"
        );
        assert_eq!(
            col.flag, expected_flag,
            "col {i_val} (vx={vx}, vz={vz}): flag mismatch"
        );
    }
}
