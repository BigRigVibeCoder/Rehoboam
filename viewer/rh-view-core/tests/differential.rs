//! Differential oracle mesher tests (SPR-033 T-02).
//!
//! Compares the binary greedy mesher against an independent naive 1x1 oracle mesher
//! over 1,000 fixed-seed SplitMix64 chunks.
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use binary_greedy_meshing as bgm;
use rh_view_core::chunk::{CS, Chunk, ChunkCoord};
use rh_view_core::mesh::mesh_chunk;
use rh_view_core::palette;
use std::collections::BTreeSet;

/// SplitMix64 pseudo-random generator.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// A visible 1x1 face: (voxel_x, voxel_y, voxel_z, face_idx, material_id).
type Face1x1 = (u8, u8, u8, u8, u16);

/// Naive oracle: emits one 1x1 face per visible voxel face.
fn naive_mesh_oracle(chunk: &Chunk) -> BTreeSet<Face1x1> {
    let mut faces = BTreeSet::new();

    for y in 0..CS {
        for x in 0..CS {
            for z in 0..CS {
                let v = chunk.get(x, y, z);
                if v == palette::AIR {
                    continue;
                }

                let px = x + 1;
                let py = y + 1;
                let pz = z + 1;

                // 6 directions:
                // 0 = Up (+Y), 1 = Down (-Y), 2 = Right (+X), 3 = Left (-X), 4 = Front (+Z), 5 = Back (-Z)
                let neighbours = [
                    (chunk.get_padded(px, py + 1, pz), 0u8), // Up
                    (chunk.get_padded(px, py - 1, pz), 1u8), // Down
                    (chunk.get_padded(px + 1, py, pz), 2u8), // Right
                    (chunk.get_padded(px - 1, py, pz), 3u8), // Left
                    (chunk.get_padded(px, py, pz + 1), 4u8), // Front
                    (chunk.get_padded(px, py, pz - 1), 5u8), // Back
                ];

                for (nb, face_idx) in neighbours {
                    let is_visible = nb == palette::AIR || (v != nb && palette::is_transparent(nb));
                    if is_visible {
                        faces.insert((x as u8, y as u8, z as u8, face_idx, v));
                    }
                }
            }
        }
    }

    faces
}

/// Decomposes greedy quads into individual 1x1 faces.
fn decompose_greedy_quads(mesh: &rh_view_core::mesh::ChunkMesh) -> (BTreeSet<Face1x1>, usize) {
    let mut faces = BTreeSet::new();
    let mut total_1x1_faces = 0;

    for (face_idx, q_list) in mesh.quads.iter().enumerate() {
        let Ok(face_u8) = u8::try_from(face_idx) else {
            continue;
        };
        let face = bgm::Face::from(face_u8);
        for &quad in q_list {
            #[allow(clippy::cast_possible_truncation, reason = "voxel id fits in u16")]
            let material = quad.voxel_id() as u16;
            let raw_verts = face.vertices_packed(quad);

            let mut min_x = 64;
            let mut max_x = 0;
            let mut min_y = 64;
            let mut max_y = 0;
            let mut min_z = 64;
            let mut max_z = 0;

            for v in &raw_verts {
                let [vx, vy, vz] = v.xyz();
                min_x = min_x.min(vx);
                max_x = max_x.max(vx);
                min_y = min_y.min(vy);
                max_y = max_y.max(vy);
                min_z = min_z.min(vz);
                max_z = max_z.max(vz);
            }

            match face_u8 {
                0 => {
                    // Up (+Y): voxel at min_y - 1
                    let vy = (min_y - 1) as u8;
                    for vx in min_x..max_x {
                        for vz in min_z..max_z {
                            total_1x1_faces += 1;
                            faces.insert((vx as u8, vy, vz as u8, 0, material));
                        }
                    }
                }
                1 => {
                    // Down (-Y): voxel at min_y
                    let vy = min_y as u8;
                    for vx in min_x..max_x {
                        for vz in min_z..max_z {
                            total_1x1_faces += 1;
                            faces.insert((vx as u8, vy, vz as u8, 1, material));
                        }
                    }
                }
                2 => {
                    // Right (+X): voxel at min_x - 1
                    let vx = (min_x - 1) as u8;
                    for vy in min_y..max_y {
                        for vz in min_z..max_z {
                            total_1x1_faces += 1;
                            faces.insert((vx, vy as u8, vz as u8, 2, material));
                        }
                    }
                }
                3 => {
                    // Left (-X): voxel at min_x
                    let vx = min_x as u8;
                    for vy in min_y..max_y {
                        for vz in min_z..max_z {
                            total_1x1_faces += 1;
                            faces.insert((vx, vy as u8, vz as u8, 3, material));
                        }
                    }
                }
                4 => {
                    // Front (+Z): voxel at min_z - 1
                    let vz = (min_z - 1) as u8;
                    for vx in min_x..max_x {
                        for vy in min_y..max_y {
                            total_1x1_faces += 1;
                            faces.insert((vx as u8, vy as u8, vz, 4, material));
                        }
                    }
                }
                5 => {
                    // Back (-Z): voxel at min_z
                    let vz = min_z as u8;
                    for vx in min_x..max_x {
                        for vy in min_y..max_y {
                            total_1x1_faces += 1;
                            faces.insert((vx as u8, vy as u8, vz, 5, material));
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    (faces, total_1x1_faces)
}

#[test]
fn test_differential_oracle() {
    let mut rng = SplitMix64(0x5011_E7E5_7E88_0033);
    let mut mesher = bgm::Mesher::<{ CS }>::new();

    let num_chunks = 1000;
    #[cfg(feature = "mutant-mesh-drop-face")]
    let mut mutant_triggered = false;

    for i in 0..num_chunks {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, i));

        // Density between 0% and 100%
        let density_pct = rng.below(101) as u32; // 0..=100
        // Max material count between 1 and 16
        let max_mat = 1 + rng.below(16) as u16; // 1..=16
        let include_water = (rng.below(2)) == 1;

        // Populate a sub-region or scattered voxels to cover diverse geometry
        // Use random box clusters or point density
        let mode = rng.below(4);
        match mode {
            0 => {
                // Point-wise probability
                if density_pct > 0 {
                    // Sample every voxel in a smaller sub-volume or full chunk
                    let bound = 16.min(CS);
                    for x in 0..bound {
                        for y in 0..bound {
                            for z in 0..bound {
                                if (rng.below(100) as u32) < density_pct {
                                    let mut mat = 1 + (rng.below(max_mat as u64) as u16);
                                    if !include_water && mat == palette::WATER {
                                        mat = palette::STONE;
                                    }
                                    chunk.set(x, y, z, mat);
                                }
                            }
                        }
                    }
                }
            }
            1 => {
                // Random solid boxes
                let boxes = rng.below(5) as usize;
                for _ in 0..boxes {
                    let bx = rng.below(45) as usize;
                    let by = rng.below(45) as usize;
                    let bz = rng.below(45) as usize;
                    let bw = 1 + rng.below(15) as usize;
                    let bh = 1 + rng.below(15) as usize;
                    let bl = 1 + rng.below(15) as usize;
                    let mut mat = 1 + (rng.below(max_mat as u64) as u16);
                    if !include_water && mat == palette::WATER {
                        mat = palette::STONE;
                    }
                    for x in bx..(bx + bw).min(CS) {
                        for y in by..(by + bh).min(CS) {
                            for z in bz..(bz + bl).min(CS) {
                                chunk.set(x, y, z, mat);
                            }
                        }
                    }
                }
            }
            2 => {
                // Horizontal slabs (e.g. terrain layers)
                let height = rng.below(30) as usize;
                for y in 0..height {
                    let mat = if y < 2 && include_water {
                        palette::WATER
                    } else {
                        palette::DIRT
                    };
                    for x in 0..20 {
                        for z in 0..20 {
                            chunk.set(x, y, z, mat);
                        }
                    }
                }
            }
            _ => {
                // Slabs with tunnels or holes
                let height = 5 + rng.below(20) as usize;
                for y in 0..height {
                    for x in 10..30 {
                        for z in 10..30 {
                            if (x + z) % 3 != 0 {
                                chunk.set(x, y, z, palette::ROCK);
                            }
                        }
                    }
                }
            }
        }

        let oracle_faces = naive_mesh_oracle(&chunk);
        let mesh = mesh_chunk(&chunk, &mut mesher);
        let (greedy_faces, total_1x1) = decompose_greedy_quads(&mesh);

        #[cfg(feature = "mutant-mesh-drop-face")]
        {
            if greedy_faces.len() != oracle_faces.len() || total_1x1 != oracle_faces.len() {
                mutant_triggered = true;
                break;
            }
        }

        #[cfg(not(feature = "mutant-mesh-drop-face"))]
        {
            // 1. Exact count (no overlap)
            assert_eq!(
                total_1x1,
                oracle_faces.len(),
                "chunk {i}: greedy 1x1 face count ({total_1x1}) != oracle count ({})",
                oracle_faces.len()
            );

            // 2. Exact set (no gaps, materials & directions match)
            assert_eq!(
                greedy_faces, oracle_faces,
                "chunk {i}: greedy decomposed face set != oracle set"
            );
        }
    }

    #[cfg(feature = "mutant-mesh-drop-face")]
    {
        assert!(
            mutant_triggered,
            "FAIL mutant-mesh-drop-face: mutant did not cause a mismatch"
        );
        panic!("FAIL mutant-mesh-drop-face: face count mismatch");
    }
}
