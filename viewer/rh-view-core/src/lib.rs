//! `rh-view-core`: Bevy-free core voxel processing, greedy meshing, terrain voxelization, and digests.

#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]

#[cfg(test)]
mod tests {
    #[test]
    fn test_workspace_init() {
        assert_eq!(2 + 2, 4);
    }
}
