//! Content and mesh digests using XXH3-128 (SPR-033 T-02).

use crate::chunk::{CS, Chunk};
use crate::mesh::ChunkMesh;
use twox_hash::XxHash3_128;

pub const MESHER_VERSION: &str = "binary-greedy-meshing-0.5.2";
pub const PACKING_VERSION: &str = "packing_v0";

/// Definition of a focus bubble in world space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BubbleDef {
    pub focus_settlement: u32,
    pub centre: [i64; 3],
    pub radius: u32,
}

/// Computes the XXH3-128 content digest over generator version, bubble definition,
/// and sorted L0 chunks' coordinates and interior voxels.
#[must_use]
pub fn compute_content_digest(
    generator_version: &str,
    bubble: &BubbleDef,
    chunks: &[Chunk],
) -> String {
    let mut buf = Vec::new();

    // 1. Generator version
    let Ok(gen_len) = u64::try_from(generator_version.len()) else {
        return String::new();
    };
    buf.extend_from_slice(&gen_len.to_le_bytes());
    buf.extend_from_slice(generator_version.as_bytes());

    // 2. Bubble definition
    buf.extend_from_slice(&bubble.focus_settlement.to_le_bytes());
    for &c in &bubble.centre {
        buf.extend_from_slice(&c.to_le_bytes());
    }
    buf.extend_from_slice(&bubble.radius.to_le_bytes());

    // 3. Number of chunks
    let Ok(chunk_count) = u64::try_from(chunks.len()) else {
        return String::new();
    };
    buf.extend_from_slice(&chunk_count.to_le_bytes());

    // 4. Chunks in coordinate order
    let mut sorted_chunks: Vec<&Chunk> = chunks.iter().collect();
    sorted_chunks.sort_by_key(|c| c.coord);

    for chunk in sorted_chunks {
        buf.push(chunk.coord.level);
        buf.extend_from_slice(&chunk.coord.x.to_le_bytes());
        buf.extend_from_slice(&chunk.coord.y.to_le_bytes());
        buf.extend_from_slice(&chunk.coord.z.to_le_bytes());

        // Interior voxels in ZXY order
        for y in 0..CS {
            for x in 0..CS {
                for z in 0..CS {
                    buf.extend_from_slice(&chunk.get(x, y, z).to_le_bytes());
                }
            }
        }
    }

    let hash_val = XxHash3_128::oneshot(&buf);
    format!("{hash_val:032x}")
}

/// Computes the XXH3-128 mesh digest over mesher version, packing version,
/// and sorted chunk meshes (quads and packed vertex buffers).
#[must_use]
pub fn compute_mesh_digest(meshes: &[ChunkMesh]) -> String {
    let mut buf = Vec::new();

    // 1. Versions
    let Ok(mesher_len) = u64::try_from(MESHER_VERSION.len()) else {
        return String::new();
    };
    buf.extend_from_slice(&mesher_len.to_le_bytes());
    buf.extend_from_slice(MESHER_VERSION.as_bytes());

    let Ok(packing_len) = u64::try_from(PACKING_VERSION.len()) else {
        return String::new();
    };
    buf.extend_from_slice(&packing_len.to_le_bytes());
    buf.extend_from_slice(PACKING_VERSION.as_bytes());

    // 2. Meshes in coordinate order
    let mut sorted_meshes: Vec<&ChunkMesh> = meshes.iter().collect();
    sorted_meshes.sort_by_key(|m| m.coord);

    let Ok(mesh_count) = u64::try_from(sorted_meshes.len()) else {
        return String::new();
    };
    buf.extend_from_slice(&mesh_count.to_le_bytes());

    for mesh in sorted_meshes {
        buf.push(mesh.coord.level);
        buf.extend_from_slice(&mesh.coord.x.to_le_bytes());
        buf.extend_from_slice(&mesh.coord.y.to_le_bytes());
        buf.extend_from_slice(&mesh.coord.z.to_le_bytes());

        for (face_idx, q_list) in mesh.quads.iter().enumerate() {
            let Ok(f_idx) = u32::try_from(face_idx) else {
                continue;
            };
            let Ok(q_len) = u32::try_from(q_list.len()) else {
                continue;
            };
            buf.extend_from_slice(&f_idx.to_le_bytes());
            buf.extend_from_slice(&q_len.to_le_bytes());
            for quad in q_list {
                buf.extend_from_slice(&quad.0.to_le_bytes());
            }
        }

        let Ok(v_len) = u32::try_from(mesh.vertices.len()) else {
            continue;
        };
        buf.extend_from_slice(&v_len.to_le_bytes());
        for &vert in &mesh.vertices {
            buf.extend_from_slice(&vert.to_le_bytes());
        }
    }

    let hash_val = XxHash3_128::oneshot(&buf);
    format!("{hash_val:032x}")
}
