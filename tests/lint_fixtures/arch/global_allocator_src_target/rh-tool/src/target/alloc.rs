//! Fixture (VER-SPR-009-TEST GAP-21): a counting allocator under src/target/, which is source,
//! not build output.
#[global_allocator]
static A: std::alloc::System = std::alloc::System;
