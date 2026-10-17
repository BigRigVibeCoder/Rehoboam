//! Fixture: a counting allocator outside rh-bench.
#[global_allocator]
static A: std::alloc::System = std::alloc::System;
