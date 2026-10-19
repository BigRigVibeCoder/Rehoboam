//! Unit and integration tests for WorldgenExport (SPR-033 T-03).
//!
//! Validates:
//! 1. Export loading and validation across worlds.
//! 2. Refusal of invalid formats, corrupted scale constants, and truncated rasters.
//! 3. SHA-256 verification of exported rasters against `specs/worldgen/golden/worldgen_lite_v0.txt`.
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use std::fs;
use std::path::Path;

use rh_view_core::export::{ExportError, WorldgenExport};
use sha2::{Digest, Sha256};

fn ensure_exports(root: &Path) {
    let export_dir = root.join("target/viewer/exports");
    assert!(
        export_dir.join("hamlet.toml").exists()
            && export_dir.join("solo.toml").exists()
            && export_dir.join("household.toml").exists()
            && export_dir.join("twin_vales.toml").exists(),
        "exports not found in {}: run export_worldgen_lite.py for hamlet, solo, household, and twin_vales first",
        export_dir.display()
    );
}

fn compute_sha256_u16_le(data: &[u16]) -> String {
    let mut hasher = Sha256::new();
    for &val in data {
        hasher.update(val.to_le_bytes());
    }
    let result = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in result {
        use std::fmt::Write;
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

#[test]
fn test_exported_rasters_match_golden_sha256() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    ensure_exports(root);

    let golden_file = root.join("specs/worldgen/golden/worldgen_lite_v0.txt");
    let golden_text = fs::read_to_string(&golden_file)
        .unwrap_or_else(|e| panic!("failed to read golden file {}: {e}", golden_file.display()));

    let export_dir = root.join("target/viewer/exports");

    // Case 1: hamlet (desert_fringe 42 256 4)
    let hamlet = WorldgenExport::load_from_dir(&export_dir, "hamlet").unwrap();
    let elev_sha = compute_sha256_u16_le(&hamlet.elevation);
    let reg_sha = compute_sha256_u16_le(&hamlet.regions);

    assert!(golden_text.contains(&format!("elevation_sha256 {elev_sha}")));
    assert!(golden_text.contains(&format!("regions_sha256 {reg_sha}")));

    // Case 2: solo (flat 1 64 1)
    let solo = WorldgenExport::load_from_dir(&export_dir, "solo").unwrap();
    let solo_elev_sha = compute_sha256_u16_le(&solo.elevation);
    let solo_reg_sha = compute_sha256_u16_le(&solo.regions);

    assert!(golden_text.contains(&format!("elevation_sha256 {solo_elev_sha}")));
    assert!(golden_text.contains(&format!("regions_sha256 {solo_reg_sha}")));

    // Case 3: household (flat 7 64 1)
    let household = WorldgenExport::load_from_dir(&export_dir, "household").unwrap();
    let hh_elev_sha = compute_sha256_u16_le(&household.elevation);
    let hh_reg_sha = compute_sha256_u16_le(&household.regions);

    assert!(golden_text.contains(&format!("elevation_sha256 {hh_elev_sha}")));
    assert!(golden_text.contains(&format!("regions_sha256 {hh_reg_sha}")));
}

#[test]
fn test_export_refusal_corrupted_format() {
    let toml_corrupt = "format = \"wrong-format-v9\"\n";
    let res = WorldgenExport::parse_and_validate(toml_corrupt, &[], &[]);
    match res {
        Err(ExportError::InvalidFormat(fmt)) => assert_eq!(fmt, "wrong-format-v9"),
        other => panic!("expected InvalidFormat, got {other:?}"),
    }
}

#[test]
fn test_export_refusal_corrupted_scale() {
    let toml_corrupt = r#"
format = "rehoboam-worldgen-lite-export-v0"
templates_version = "v0"
case = "c"
world = "w"
manifest_path = "p"
manifest_sha256 = "s"
raster_resolution = 64
sea_level = 20000
land_count = 100
seats = []
edges = []
routes = []

[scale]
metres_per_cell = 999
elevation_units_per_metre = 64
metres_per_voxel_num = 7
metres_per_voxel_den = 20
voxels_per_cell_num = 600
voxels_per_cell_den = 7
elevation_units_per_voxel_num = 2240
elevation_units_per_voxel_den = 100
"#;
    let res = WorldgenExport::parse_and_validate(toml_corrupt, &[], &[]);
    match res {
        Err(ExportError::InvalidScaleConstant {
            name,
            expected,
            found,
        }) => {
            assert_eq!(name, "metres_per_cell");
            assert_eq!(expected, 30);
            assert_eq!(found, 999);
        }
        other => panic!("expected InvalidScaleConstant, got {other:?}"),
    }
}

#[test]
fn test_export_refusal_mismatched_raster_length() {
    let toml_valid = r#"
format = "rehoboam-worldgen-lite-export-v0"
templates_version = "v0"
case = "c"
world = "w"
manifest_path = "p"
manifest_sha256 = "s"
raster_resolution = 64
sea_level = 20000
land_count = 100
seats = []
edges = []
routes = []

[scale]
metres_per_cell = 30
elevation_units_per_metre = 64
metres_per_voxel_num = 7
metres_per_voxel_den = 20
voxels_per_cell_num = 600
voxels_per_cell_den = 7
elevation_units_per_voxel_num = 2240
elevation_units_per_voxel_den = 100
"#;
    // Expected bytes for 64x64 is 64 * 64 * 2 = 8192 bytes
    let truncated_elev = vec![0u8; 100];
    let valid_reg = vec![0u8; 8192];
    let res = WorldgenExport::parse_and_validate(toml_valid, &truncated_elev, &valid_reg);
    match res {
        Err(ExportError::InvalidRasterLength {
            name,
            expected,
            found,
        }) => {
            assert_eq!(name, "elevation");
            assert_eq!(expected, 8192);
            assert_eq!(found, 100);
        }
        other => panic!("expected InvalidRasterLength, got {other:?}"),
    }
}
