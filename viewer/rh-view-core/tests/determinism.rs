//! Determinism test across runs and threads (SPR-033 T-02).
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use rh_view_core::chunk::{CS, Chunk, ChunkCoord};
use rh_view_core::digest::{BubbleDef, compute_content_digest, compute_mesh_digest};
use rh_view_core::mesh::mesh_chunks;
use rh_view_core::palette;

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

fn generate_test_chunks() -> Vec<Chunk> {
    let mut rng = SplitMix64(0xDE7E_8811_0033_0001);
    let mut chunks = Vec::new();

    for cx in -2..2 {
        for cy in -1..1 {
            for cz in -2..2 {
                let mut chunk = Chunk::new(ChunkCoord::new(0, cx, cy, cz));
                for x in 0..CS {
                    for z in 0..CS {
                        let height = (rng.below(40) as usize).min(CS);
                        for y in 0..height {
                            let mat = if y == 0 {
                                palette::WATER
                            } else if y < 5 {
                                palette::SAND
                            } else if y < 25 {
                                palette::DIRT
                            } else {
                                palette::GRASS
                            };
                            chunk.set(x, y, z, mat);
                        }
                    }
                }
                chunks.push(chunk);
            }
        }
    }

    chunks
}

#[test]
fn test_meshing_determinism_across_runs_and_threads() {
    let chunks = generate_test_chunks();
    let bubble = BubbleDef {
        focus_settlement: 1,
        centre: [0, 0, 0],
        radius: 90,
    };

    // Run 1: 1 thread
    let content_digest_1 = compute_content_digest("terrain_v0", &bubble, &chunks);
    let meshes_1t_run1 = mesh_chunks(&chunks, 1);
    let mesh_digest_1t_run1 = compute_mesh_digest(&meshes_1t_run1);

    // Run 2: 1 thread
    let content_digest_2 = compute_content_digest("terrain_v0", &bubble, &chunks);
    let meshes_1t_run2 = mesh_chunks(&chunks, 1);
    let mesh_digest_1t_run2 = compute_mesh_digest(&meshes_1t_run2);

    assert_eq!(
        content_digest_1, content_digest_2,
        "content digest must be equal across runs"
    );
    assert_eq!(
        mesh_digest_1t_run1, mesh_digest_1t_run2,
        "mesh digest must be equal across runs (1 thread)"
    );

    // Run 3: 4 threads
    let meshes_4t_run1 = mesh_chunks(&chunks, 4);
    let mesh_digest_4t_run1 = compute_mesh_digest(&meshes_4t_run1);

    // Run 4: 4 threads
    let meshes_4t_run2 = mesh_chunks(&chunks, 4);
    let mesh_digest_4t_run2 = compute_mesh_digest(&meshes_4t_run2);

    assert_eq!(
        mesh_digest_4t_run1, mesh_digest_4t_run2,
        "mesh digest must be equal across runs (4 threads)"
    );
    assert_eq!(
        mesh_digest_1t_run1, mesh_digest_4t_run1,
        "mesh digest must be equal between 1 and 4 threads"
    );

    // Check individual chunk vertices and quads equality between 1 and 4 threads
    assert_eq!(meshes_1t_run1.len(), meshes_4t_run1.len());
    for (m1, m4) in meshes_1t_run1.iter().zip(meshes_4t_run1.iter()) {
        assert_eq!(m1.coord, m4.coord);
        assert_eq!(m1.quads, m4.quads);
        assert_eq!(m1.vertices, m4.vertices);
        assert_eq!(m1.indices, m4.indices);
    }
}
