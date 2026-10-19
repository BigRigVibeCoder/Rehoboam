//! Integration tests for terrain golden digests, determinism, and LOD reduction (SPR-033 T-03).
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use rh_view_core::chunk::ChunkCoord;
use rh_view_core::digest::{BubbleDef, compute_content_digest};
use rh_view_core::export::WorldgenExport;
use rh_view_core::lod::reduce_chunk;
use rh_view_core::terrain::{GENERATOR_VERSION, TerrainContext};

fn find_repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../..").canonicalize().unwrap()
}

fn ensure_hamlet_export(root: &Path) -> PathBuf {
    let export_dir = root.join("target/viewer/exports");
    let hamlet_toml = export_dir.join("hamlet.toml");
    assert!(
        hamlet_toml.exists(),
        "hamlet export not found at {}: run export_worldgen_lite.py --world hamlet first",
        hamlet_toml.display()
    );
    export_dir
}

fn parse_golden(path: &Path) -> BTreeMap<String, String> {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("failed to read golden file {}: {e}", path.display()));
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        if let Some(key) = parts.next() {
            let val = parts.collect::<Vec<_>>().join(" ");
            map.insert(key.to_string(), val);
        }
    }
    map
}

#[test]
fn test_terrain_golden_digests_match() {
    let root = find_repo_root();
    let export_dir = ensure_hamlet_export(&root);
    let export = WorldgenExport::load_from_dir(&export_dir, "hamlet")
        .unwrap_or_else(|e| panic!("failed to load hamlet export: {e}"));

    let ctx = TerrainContext::new(&export);

    let bubble = BubbleDef {
        focus_settlement: 1,
        centre: [13585, 1026, 6042],
        radius: 90,
    };

    let l0_chunks = ctx.generate_bubble_chunks(&bubble, 0);
    let l0_digest = compute_content_digest(GENERATOR_VERSION, &bubble, &l0_chunks);

    let l1_chunks = ctx.generate_bubble_chunks(&bubble, 1);
    let l1_digest = compute_content_digest(GENERATOR_VERSION, &bubble, &l1_chunks);

    let golden_path = root.join("viewer/golden/terrain_v0.txt");
    let golden = parse_golden(&golden_path);

    #[cfg(not(feature = "mutant-terrain-off-by-one"))]
    {
        assert_eq!(
            golden.get("version").map(String::as_str),
            Some(GENERATOR_VERSION),
            "generator version must match golden"
        );
        assert_eq!(
            golden.get("l0_digest").map(String::as_str),
            Some(l0_digest.as_str()),
            "l0 content digest must match golden"
        );
        assert_eq!(
            golden.get("l1_digest").map(String::as_str),
            Some(l1_digest.as_str()),
            "l1 content digest must match golden"
        );
    }

    #[cfg(feature = "mutant-terrain-off-by-one")]
    {
        if golden.get("l0_digest").map(String::as_str) != Some(l0_digest.as_str())
            || golden.get("l1_digest").map(String::as_str) != Some(l1_digest.as_str())
        {
            panic!(
                "FAIL mutant-terrain-off-by-one: height rounding shifted by one caused digest mismatch"
            );
        }
    }
}

#[test]
fn test_terrain_determinism_across_runs_and_threads() {
    let root = find_repo_root();
    let export_dir = ensure_hamlet_export(&root);
    let export = WorldgenExport::load_from_dir(&export_dir, "hamlet")
        .unwrap_or_else(|e| panic!("failed to load hamlet export: {e}"));

    let ctx = TerrainContext::new(&export);

    let bubble = BubbleDef {
        focus_settlement: 1,
        centre: [13585, 1026, 6042],
        radius: 90,
    };

    // Run 1: sequential
    let l0_run1 = ctx.generate_bubble_chunks(&bubble, 0);
    let digest_run1 = compute_content_digest(GENERATOR_VERSION, &bubble, &l0_run1);

    // Run 2: sequential
    let l0_run2 = ctx.generate_bubble_chunks(&bubble, 0);
    let digest_run2 = compute_content_digest(GENERATOR_VERSION, &bubble, &l0_run2);

    assert_eq!(
        digest_run1, digest_run2,
        "determinism across sequential runs"
    );

    // Run 3: 4 threads evaluating chunks concurrently using scoped threads
    let coords: Vec<ChunkCoord> = l0_run1.iter().map(|c| c.coord).collect();
    let ctx_ref = &ctx;
    let thread_chunks = std::thread::scope(|s| {
        let handles: Vec<_> = (0..4)
            .map(|t| {
                let chunk_coords: Vec<ChunkCoord> = coords
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i % 4 == t)
                    .map(|(_, &c)| c)
                    .collect();
                s.spawn(move || {
                    let mut chunks = Vec::new();
                    for coord in chunk_coords {
                        chunks.push(ctx_ref.generate_chunk(coord));
                    }
                    chunks
                })
            })
            .collect();

        let mut collected = Vec::new();
        for h in handles {
            collected.extend(h.join().unwrap());
        }
        collected.sort_by_key(|c| c.coord);
        collected
    });

    let digest_threaded = compute_content_digest(GENERATOR_VERSION, &bubble, &thread_chunks);
    assert_eq!(
        digest_run1, digest_threaded,
        "determinism across 1 and 4 thread chunk generation"
    );
}

#[test]
fn test_lod_reduction_equivalence() {
    let root = find_repo_root();
    let export_dir = ensure_hamlet_export(&root);
    let export = WorldgenExport::load_from_dir(&export_dir, "hamlet")
        .unwrap_or_else(|e| panic!("failed to load hamlet export: {e}"));

    let ctx = TerrainContext::new(&export);

    // Test equivalence on a central L1 chunk covering settlement 1
    // Centre in coarse voxels: 6792, 513, 3021
    // Chunk coord at L1: 6792/62 = 109, 513/62 = 8, 3021/62 = 48
    let l1_coord = ChunkCoord::new(1, 109, 8, 48);
    let on_demand_l1 = ctx.generate_chunk(l1_coord);

    // Corresponding 8 L0 chunks:
    let base_x = 109 * 2;
    let base_y = 8 * 2;
    let base_z = 48 * 2;

    let mut l0_chunks = Vec::new();
    for dy in 0..2 {
        for dz in 0..2 {
            for dx in 0..2 {
                let coord = ChunkCoord::new(0, base_x + dx, base_y + dy, base_z + dz);
                l0_chunks.push(ctx.generate_chunk(coord));
            }
        }
    }

    let l0_refs: [&rh_view_core::chunk::Chunk; 8] = [
        &l0_chunks[0],
        &l0_chunks[1],
        &l0_chunks[2],
        &l0_chunks[3],
        &l0_chunks[4],
        &l0_chunks[5],
        &l0_chunks[6],
        &l0_chunks[7],
    ];

    let reduced_l1 = reduce_chunk(l0_refs, l1_coord);

    // Verify interior voxels match exactly
    for y in 0..rh_view_core::chunk::CS {
        for z in 0..rh_view_core::chunk::CS {
            for x in 0..rh_view_core::chunk::CS {
                let expected = on_demand_l1.get(x, y, z);
                let actual = reduced_l1.get(x, y, z);
                assert_eq!(
                    actual, expected,
                    "interior voxel mismatch at ({x}, {y}, {z})"
                );
            }
        }
    }
}
