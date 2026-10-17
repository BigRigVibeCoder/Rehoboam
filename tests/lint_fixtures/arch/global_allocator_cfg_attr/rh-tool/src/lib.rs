//! Fixture: a counting allocator behind cfg_attr, outside rh-bench.
#[cfg_attr(feature = "x", global_allocator)]
static A: std::alloc::System = std::alloc::System;
