//! Binary greedy mesher adapter and 32-bit packed vertices (SPR-033 T-02).

use crate::chunk::{CS, CS_P, CS_P2, Chunk, ChunkCoord};
use crate::palette;
use binary_greedy_meshing as bgm;

/// Packed 32-bit vertex:
/// - 6 bits position X (0..63)
/// - 6 bits position Y (0..63)
/// - 6 bits position Z (0..63)
/// - 3 bits face index (0..5)
/// - 11 bits material ID (0..2047)
#[inline]
#[must_use]
pub fn pack_vertex(x: u32, y: u32, z: u32, face: u32, material: u16) -> u32 {
    (x & 0x3F)
        | ((y & 0x3F) << 6)
        | ((z & 0x3F) << 12)
        | ((face & 0x7) << 18)
        | ((u32::from(material) & 0x7FF) << 21)
}

/// Unpacks a 32-bit vertex into (x, y, z, face, material).
#[inline]
#[must_use]
pub fn unpack_vertex(packed: u32) -> (u32, u32, u32, u32, u16) {
    let x = packed & 0x3F;
    let y = (packed >> 6) & 0x3F;
    let z = (packed >> 12) & 0x3F;
    let face = (packed >> 18) & 0x7;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "material id mask is 11 bits, fits in u16"
    )]
    let material = ((packed >> 21) & 0x7FF) as u16;
    (x, y, z, face, material)
}

/// The meshed output for a chunk.
#[derive(Debug, Clone)]
pub struct ChunkMesh {
    /// Coordinate of the meshed chunk.
    pub coord: ChunkCoord,
    /// Greedy quads per face (0 = Up, 1 = Down, 2 = Right, 3 = Left, 4 = Front, 5 = Back).
    pub quads: [Vec<bgm::Quad>; 6],
    /// Packed 32-bit vertex buffer.
    pub vertices: Vec<u32>,
    /// Index buffer for triangle list.
    pub indices: Vec<u32>,
}

/// Builds opaque and transparent masks for a padded chunk buffer.
#[must_use]
pub fn build_masks(chunk: &Chunk) -> (Box<[u64]>, Box<[u64]>) {
    let mut opaque_mask = vec![0u64; CS_P2].into_boxed_slice();
    let mut trans_mask = vec![0u64; CS_P2].into_boxed_slice();

    for py in 0..CS_P {
        for px in 0..CS_P {
            let r = px + py * CS_P;
            let mut op_bits = 0u64;
            let mut tr_bits = 0u64;
            for pz in 0..CS_P {
                let v = chunk.get_padded(px, py, pz);
                if palette::is_opaque(v) {
                    op_bits |= 1 << pz;
                } else if palette::is_transparent(v) {
                    tr_bits |= 1 << pz;
                }
            }
            opaque_mask[r] = op_bits;
            trans_mask[r] = tr_bits;
        }
    }

    (opaque_mask, trans_mask)
}

/// Meshes a single chunk using the provided Mesher buffer.
pub fn mesh_chunk(chunk: &Chunk, mesher: &mut bgm::Mesher<CS>) -> ChunkMesh {
    mesher.clear();
    let (opaque_mask, trans_mask) = build_masks(chunk);
    mesher.fast_mesh(chunk.padded_voxels(), &opaque_mask, &trans_mask);

    #[allow(unused_mut, reason = "mutant-mesh-drop-face modifies quads")]
    let mut quads: [Vec<bgm::Quad>; 6] = core::array::from_fn(|i| mesher.quads[i].clone());

    #[cfg(feature = "mutant-mesh-drop-face")]
    {
        for q_list in &mut quads {
            if !q_list.is_empty() {
                q_list.pop();
                break;
            }
        }
    }

    let mut total_quads = 0;
    for q_list in &quads {
        total_quads += q_list.len();
    }

    let mut vertices = Vec::with_capacity(total_quads * 4);
    for (face_idx, q_list) in quads.iter().enumerate() {
        let Ok(face_u8) = u8::try_from(face_idx) else {
            continue;
        };
        let Ok(face_u32) = u32::try_from(face_idx) else {
            continue;
        };
        let face = bgm::Face::from(face_u8);
        for &quad in q_list {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "voxel id fits in u16 in palette_v0"
            )]
            let material = quad.voxel_id() as u16;
            let raw_verts = face.vertices_packed(quad);
            for v in &raw_verts {
                let [vx, vy, vz] = v.xyz();
                vertices.push(pack_vertex(vx, vy, vz, face_u32, material));
            }
        }
    }

    let indices = bgm::indices(total_quads);

    ChunkMesh {
        coord: chunk.coord,
        quads,
        vertices,
        indices,
    }
}

/// Meshes a list of chunks using N threads, preserving chunk coordinate order.
#[must_use]
pub fn mesh_chunks(chunks: &[Chunk], num_threads: usize) -> Vec<ChunkMesh> {
    if chunks.is_empty() {
        return Vec::new();
    }

    let mut sorted_indices: Vec<usize> = (0..chunks.len()).collect();
    sorted_indices.sort_by_key(|&i| chunks[i].coord);

    if num_threads <= 1 || chunks.len() <= 1 {
        let mut mesher = bgm::Mesher::<CS>::new();
        return sorted_indices
            .into_iter()
            .map(|i| mesh_chunk(&chunks[i], &mut mesher))
            .collect();
    }

    let chunk_count = chunks.len();
    let num_workers = num_threads.min(chunk_count);
    let chunk_size = chunk_count.div_ceil(num_workers);

    let mut results: Vec<ChunkMesh> = Vec::with_capacity(chunk_count);
    let chunks_ref = chunks;

    std::thread::scope(|s| {
        let mut handles = Vec::new();
        for worker_idx in 0..num_workers {
            let start = worker_idx * chunk_size;
            let end = (start + chunk_size).min(chunk_count);
            if start >= end {
                continue;
            }
            let slice_indices = &sorted_indices[start..end];
            let handle = s.spawn(move || {
                let mut mesher = bgm::Mesher::<CS>::new();
                let mut worker_results = Vec::with_capacity(slice_indices.len());
                for &idx in slice_indices {
                    worker_results.push(mesh_chunk(&chunks_ref[idx], &mut mesher));
                }
                worker_results
            });
            handles.push(handle);
        }

        for handle in handles {
            if let Ok(worker_res) = handle.join() {
                results.extend(worker_res);
            }
        }
    });

    results
}
