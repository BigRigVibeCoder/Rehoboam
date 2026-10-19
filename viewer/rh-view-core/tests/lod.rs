//! Unit tests for Level-of-Detail reduction (SPR-033 T-02, A7).
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use rh_view_core::lod::reduce_voxel;
use rh_view_core::palette;

#[test]
fn test_all_empty_and_all_solid() {
    // All empty (air) -> air
    let all_air = [palette::AIR; 8];
    assert_eq!(reduce_voxel(&all_air), palette::AIR);

    // All solid -> same material
    let all_rock = [palette::ROCK; 8];
    assert_eq!(reduce_voxel(&all_rock), palette::ROCK);

    let all_dirt = [palette::DIRT; 8];
    assert_eq!(reduce_voxel(&all_dirt), palette::DIRT);
}

#[test]
fn test_top_layer_and_tie_rules() {
    // Top layer non-air overrides bottom layer:
    // Bottom layer (indices 0..4) is all DIRT (4 voxels).
    // Top layer (indices 4..8) has 1 GRASS and 3 AIR.
    let mut children = [palette::AIR; 8];
    children[0] = palette::DIRT;
    children[1] = palette::DIRT;
    children[2] = palette::DIRT;
    children[3] = palette::DIRT;
    children[4] = palette::GRASS; // dy = 1, top layer
    // Under surface-preserving rule, GRASS wins over 4 DIRT voxels below it.
    #[cfg(not(feature = "mutant-lod-majority"))]
    assert_eq!(reduce_voxel(&children), palette::GRASS);
    #[cfg(feature = "mutant-lod-majority")]
    let _ = reduce_voxel(&children);

    // Tie breaking rule: lowest material id wins
    // Top layer has 2 SAND (id 2) and 2 ROCK (id 5).
    let mut tie_children = [palette::AIR; 8];
    tie_children[4] = palette::ROCK; // id 5
    tie_children[5] = palette::SAND; // id 2
    tie_children[6] = palette::ROCK; // id 5
    tie_children[7] = palette::SAND; // id 2
    assert_eq!(
        reduce_voxel(&tie_children),
        palette::SAND,
        "lowest id (2 < 5) must win ties"
    );
}

#[test]
fn test_shore_case_water_and_sand() {
    // Shore case (V16):
    // Case 1: 3 water, 1 sand in top layer -> water wins (3 > 1) and is transparent
    let mut shore1 = [palette::AIR; 8];
    shore1[4] = palette::WATER;
    shore1[5] = palette::WATER;
    shore1[6] = palette::WATER;
    shore1[7] = palette::SAND;
    let r1 = reduce_voxel(&shore1);
    assert_eq!(r1, palette::WATER);
    assert!(palette::is_transparent(r1));

    // Case 2: 1 water, 3 sand in top layer -> sand wins (3 > 1) and is opaque
    let mut shore2 = [palette::AIR; 8];
    shore2[4] = palette::WATER;
    shore2[5] = palette::SAND;
    shore2[6] = palette::SAND;
    shore2[7] = palette::SAND;
    let r2 = reduce_voxel(&shore2);
    assert_eq!(r2, palette::SAND);
    assert!(palette::is_opaque(r2));

    // Case 3: 2 water, 2 sand in top layer -> tie -> water wins (id 1 < 2) and is transparent
    let mut shore3 = [palette::AIR; 8];
    shore3[4] = palette::WATER;
    shore3[5] = palette::SAND;
    shore3[6] = palette::WATER;
    shore3[7] = palette::SAND;
    let r3 = reduce_voxel(&shore3);
    assert_eq!(r3, palette::WATER);
    assert!(palette::is_transparent(r3));
}

#[test]
fn test_road_survives_l1_and_l2() {
    // Test a 1-voxel wide road of STONE on top of a surface of DIRT.
    // In a 4x4x4 block of L0 voxels:
    // y = 0: solid DIRT base
    // y = 1: 1-voxel road of STONE at x = 0 (for all z in 0..4), air elsewhere
    // y >= 2: AIR
    let mut children_l1 = [palette::AIR; 8];
    children_l1[0] = palette::DIRT;
    children_l1[1] = palette::DIRT;
    children_l1[2] = palette::DIRT;
    children_l1[3] = palette::DIRT;
    children_l1[4] = palette::STONE;
    children_l1[5] = palette::AIR;
    children_l1[6] = palette::STONE;
    children_l1[7] = palette::AIR;

    let l1_voxel = reduce_voxel(&children_l1);

    #[cfg(feature = "mutant-lod-majority")]
    {
        // Under majority mutant, DIRT (4 votes) beats STONE (2 votes) and AIR (2 votes),
        // so road is obliterated!
        if l1_voxel != palette::STONE {
            panic!("FAIL mutant-lod-majority: road lost at L1");
        }
    }

    #[cfg(not(feature = "mutant-lod-majority"))]
    {
        // Under surface-preserving rule, top-most non-air layer is dy = 1, where STONE is present!
        assert_eq!(l1_voxel, palette::STONE, "1-voxel road must survive at L1");

        let mut children_l2 = [palette::AIR; 8];
        children_l2[0] = palette::STONE;
        children_l2[1] = palette::DIRT;
        children_l2[2] = palette::STONE;
        children_l2[3] = palette::DIRT;

        let l2_voxel = reduce_voxel(&children_l2);
        assert!(
            l2_voxel != palette::AIR,
            "surface voxel must not be air at L2"
        );
    }
}
