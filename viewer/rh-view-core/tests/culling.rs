//! Unit tests for face culling and boundary padding (SPR-033 T-02).
#![allow(
    clippy::pedantic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test assertions and fixtures"
)]

use binary_greedy_meshing as bgm;
use rh_view_core::chunk::{CS, Chunk, ChunkCoord, ChunkNeighbours};
use rh_view_core::mesh::mesh_chunk;
use rh_view_core::palette;

#[test]
fn test_single_voxel() {
    let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
    chunk.set(10, 10, 10, palette::STONE);

    let mut mesher = bgm::Mesher::<{ CS }>::new();
    let mesh = mesh_chunk(&chunk, &mut mesher);

    // Single voxel has 6 faces, 1 quad per face, each 1x1
    for face in 0..6 {
        assert_eq!(mesh.quads[face].len(), 1, "face {face} should have 1 quad");
        let q = mesh.quads[face][0];
        assert_eq!(q.width(), 1);
        assert_eq!(q.height(), 1);
        assert_eq!(q.voxel_id(), palette::STONE as u64);
    }
    assert_eq!(mesh.vertices.len(), 24); // 6 faces * 4 vertices
    assert_eq!(mesh.indices.len(), 36); // 6 faces * 6 indices
}

#[test]
fn test_solid_cube() {
    let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
    // 10x10x10 cube from (10, 10, 10) to (19, 19, 19)
    for x in 10..20 {
        for y in 10..20 {
            for z in 10..20 {
                chunk.set(x, y, z, palette::DIRT);
            }
        }
    }

    let mut mesher = bgm::Mesher::<{ CS }>::new();
    let mesh = mesh_chunk(&chunk, &mut mesher);

    // Greedy mesher should merge each 10x10 face into 1 single quad
    for face in 0..6 {
        assert_eq!(
            mesh.quads[face].len(),
            1,
            "solid cube face {face} should merge into 1 quad"
        );
        let q = mesh.quads[face][0];
        assert_eq!(q.width(), 10);
        assert_eq!(q.height(), 10);
        assert_eq!(q.voxel_id(), palette::DIRT as u64);
    }
}

#[test]
fn test_hollow_box() {
    let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
    // 6x6x6 box with 1-voxel thick walls, hollow interior (4x4x4 air inside)
    for x in 10..16 {
        for y in 10..16 {
            for z in 10..16 {
                let is_wall = x == 10 || x == 15 || y == 10 || y == 15 || z == 10 || z == 15;
                if is_wall {
                    chunk.set(x, y, z, palette::ROCK);
                }
            }
        }
    }

    let mut mesher = bgm::Mesher::<{ CS }>::new();
    let mesh = mesh_chunk(&chunk, &mut mesher);

    // Each direction has exterior faces and interior faces
    for face in 0..6 {
        assert!(!mesh.quads[face].is_empty(), "face {face} must have quads");
    }
    let total_quads: usize = mesh.quads.iter().map(|q| q.len()).sum();
    assert!(
        total_quads >= 12,
        "hollow box should have exterior and interior quads"
    );
}

#[test]
fn test_one_voxel_tunnel() {
    let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
    // 3x3 block along Z with tunnel at centre:
    // (x, y) in 10..13, z in 10..20, air at (11, 11, z)
    for z in 10..20 {
        for x in 10..13 {
            for y in 10..13 {
                if x == 11 && y == 11 {
                    // air in tunnel
                    continue;
                }
                chunk.set(x, y, z, palette::STONE);
            }
        }
    }

    let mut mesher = bgm::Mesher::<{ CS }>::new();
    let mesh = mesh_chunk(&chunk, &mut mesher);

    // Tunnel walls must be visible
    let total_quads: usize = mesh.quads.iter().map(|q| q.len()).sum();
    assert!(
        total_quads > 6,
        "tunnel should have exterior and tunnel wall quads"
    );
}

#[test]
fn test_chunk_boundary_culling() {
    let mut mesher = bgm::Mesher::<{ CS }>::new();

    // 1. -X boundary (face 3 = Left)
    {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
        chunk.set(0, 10, 10, palette::GRASS);
        let m0 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m0.quads[3].len(),
            1,
            "-X boundary face must be present without neighbour"
        );

        let mut neg_x = Chunk::new(ChunkCoord::new(0, -1, 0, 0));
        neg_x.set(CS - 1, 10, 10, palette::GRASS);
        let nb = ChunkNeighbours {
            neg_x: Some(&neg_x),
            ..Default::default()
        };
        chunk.fill_padding(&nb);
        let m1 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m1.quads[3].len(),
            0,
            "-X boundary face must be culled with neighbour"
        );
    }

    // 2. +X boundary (face 2 = Right)
    {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
        chunk.set(CS - 1, 10, 10, palette::GRASS);
        let m0 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m0.quads[2].len(),
            1,
            "+X boundary face must be present without neighbour"
        );

        let mut pos_x = Chunk::new(ChunkCoord::new(0, 1, 0, 0));
        pos_x.set(0, 10, 10, palette::GRASS);
        let nb = ChunkNeighbours {
            pos_x: Some(&pos_x),
            ..Default::default()
        };
        chunk.fill_padding(&nb);
        let m1 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m1.quads[2].len(),
            0,
            "+X boundary face must be culled with neighbour"
        );
    }

    // 3. -Y boundary (face 1 = Down)
    {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
        chunk.set(10, 0, 10, palette::GRASS);
        let m0 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m0.quads[1].len(),
            1,
            "-Y boundary face must be present without neighbour"
        );

        let mut neg_y = Chunk::new(ChunkCoord::new(0, 0, -1, 0));
        neg_y.set(10, CS - 1, 10, palette::GRASS);
        let nb = ChunkNeighbours {
            neg_y: Some(&neg_y),
            ..Default::default()
        };
        chunk.fill_padding(&nb);
        let m1 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m1.quads[1].len(),
            0,
            "-Y boundary face must be culled with neighbour"
        );
    }

    // 4. +Y boundary (face 0 = Up)
    {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
        chunk.set(10, CS - 1, 10, palette::GRASS);
        let m0 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m0.quads[0].len(),
            1,
            "+Y boundary face must be present without neighbour"
        );

        let mut pos_y = Chunk::new(ChunkCoord::new(0, 0, 1, 0));
        pos_y.set(10, 0, 10, palette::GRASS);
        let nb = ChunkNeighbours {
            pos_y: Some(&pos_y),
            ..Default::default()
        };
        chunk.fill_padding(&nb);
        let m1 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m1.quads[0].len(),
            0,
            "+Y boundary face must be culled with neighbour"
        );
    }

    // 5. -Z boundary (face 5 = Back)
    {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
        chunk.set(10, 10, 0, palette::GRASS);
        let m0 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m0.quads[5].len(),
            1,
            "-Z boundary face must be present without neighbour"
        );

        let mut neg_z = Chunk::new(ChunkCoord::new(0, 0, 0, -1));
        neg_z.set(10, 10, CS - 1, palette::GRASS);
        let nb = ChunkNeighbours {
            neg_z: Some(&neg_z),
            ..Default::default()
        };
        chunk.fill_padding(&nb);
        let m1 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m1.quads[5].len(),
            0,
            "-Z boundary face must be culled with neighbour"
        );
    }

    // 6. +Z boundary (face 4 = Front)
    {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0, 0));
        chunk.set(10, 10, CS - 1, palette::GRASS);
        let m0 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m0.quads[4].len(),
            1,
            "+Z boundary face must be present without neighbour"
        );

        let mut pos_z = Chunk::new(ChunkCoord::new(0, 0, 0, 1));
        pos_z.set(10, 10, 0, palette::GRASS);
        let nb = ChunkNeighbours {
            pos_z: Some(&pos_z),
            ..Default::default()
        };
        chunk.fill_padding(&nb);
        let m1 = mesh_chunk(&chunk, &mut mesher);
        assert_eq!(
            m1.quads[4].len(),
            0,
            "+Z boundary face must be culled with neighbour"
        );
    }
}
