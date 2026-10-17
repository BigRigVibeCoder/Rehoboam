//! The bench allocators (SPR-009): V-PERF-11's counting allocator ([`Counting`]; T-05, ruling 5),
//! heap allocations per steady-state tick with buffer growth counted separately, and V-PERF-04's
//! deterministic arena ([`Arena`]), which makes instruction counts independent of where the
//! repository is checked out.
//!
//! [`Counting`] wraps the system allocator and counts every call with relaxed atomics, on every
//! thread (the engine's scoped workers included). A bench target installs it as its
//! `#[global_allocator]` and reads [`snapshot`] before and after the code it measures.
//!
//! - `allocations`: `alloc` and `alloc_zeroed` calls, a fresh block each.
//! - `growths`: `realloc` to a larger size, the amortised doubling of a growing buffer (counted
//!   separately, `docs/09` V-PERF-11).
//! - `shrinks`: `realloc` to a size no larger.
//! - `deallocations`: `dealloc` calls.
//!
//! Containment (Architect ruling, 2026-10-16): this module is compiled only with the
//! `count-alloc` or `arena-alloc` feature; only `rh-bench`'s bench targets require them, and only a
//! bench target of `rh-bench` declares a `#[global_allocator]` (`bin/lint_arch.py`: count-alloc,
//! global-allocator). It is the one file of the workspace on GOV-003 §2's unsafe allow-list
//! (`UNSAFE_ALLOW_LIST`): implementing `GlobalAlloc` needs it.
//!
//! REF: `docs/09` V-PERF-11; GOV-003 §2, §6 row 6; `CODEX/05_PROJECT/SPR-009_Benchmarks.md` T-05,
//! ruling 5; BCK-002 A-32.
#![allow(
    unsafe_code,
    reason = "GlobalAlloc is an unsafe trait; this module is GOV-003 §2's one allow-listed file"
)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU64, AtomicUsize, Ordering};

static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);
static GROWTHS: AtomicU64 = AtomicU64::new(0);
static SHRINKS: AtomicU64 = AtomicU64::new(0);
static DEALLOCATIONS: AtomicU64 = AtomicU64::new(0);

/// The system allocator, counting.
pub struct Counting;

/// The counts since the process started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Fresh blocks (`alloc`, `alloc_zeroed`).
    pub allocations: u64,
    /// Reallocations to a larger size: buffer growth.
    pub growths: u64,
    /// Reallocations to a size no larger.
    pub shrinks: u64,
    /// Blocks freed.
    pub deallocations: u64,
}

impl Snapshot {
    /// The counts from `earlier` to `self`.
    #[must_use]
    pub const fn since(self, earlier: Self) -> Self {
        Self {
            allocations: self.allocations.wrapping_sub(earlier.allocations),
            growths: self.growths.wrapping_sub(earlier.growths),
            shrinks: self.shrinks.wrapping_sub(earlier.shrinks),
            deallocations: self.deallocations.wrapping_sub(earlier.deallocations),
        }
    }
}

/// The counts now.
#[must_use]
pub fn snapshot() -> Snapshot {
    Snapshot {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        growths: GROWTHS.load(Ordering::Relaxed),
        shrinks: SHRINKS.load(Ordering::Relaxed),
        deallocations: DEALLOCATIONS.load(Ordering::Relaxed),
    }
}

// SAFETY: every method forwards its arguments unchanged to `System`, which upholds `GlobalAlloc`'s
// contract; the counters are atomics that never allocate, so counting cannot re-enter the allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the caller's guarantees for `alloc` (a non-zero-size layout) pass on unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        DEALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        // SAFETY: the caller guarantees `ptr` came from this allocator (that is, from `System`
        // through it) with `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size > layout.size() {
            GROWTHS.fetch_add(1, Ordering::Relaxed);
        } else {
            SHRINKS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: the caller's guarantees for `realloc` (`ptr` from this allocator with `layout`,
        // a valid non-zero `new_size`) pass on unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// The arena's size: one reservation of address space, whose pages are touched only as far as a
/// bench allocates. The PR-tier benches (10k rows) take between 16 and 32 MiB each, setup included
/// (measured: every case fails at a 16 MiB reservation, none at 32 MiB), so 1 GiB leaves 30 times
/// that for benches that grow. Exhausting it aborts with [`exhausted`]'s message.
const ARENA_BYTES: usize = 1 << 30;
// `exhausted` names the size in its message; keep the two together.
const _: () = assert!(
    ARENA_BYTES == 1 << 30,
    "update the size in exhausted()'s message"
);
/// Every arena block starts on, and is a multiple of, this many bytes.
const GRAIN: usize = 64;
/// The arena's own alignment (a page).
const ARENA_ALIGN: usize = 4096;

static ARENA: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static ARENA_NEXT: AtomicUsize = AtomicUsize::new(0);

/// A deterministic bump arena for the instruction-count benches (V-PERF-04).
///
/// glibc's malloc returns blocks whose alignment and internal path depend on everything the
/// process allocated before, and the benchmark harness allocates differently depending on path
/// lengths compiled into it (the checkout's directory) and passed to it (the executable's path).
/// That moved the counts of one commit by up to 13% between two checkouts (SPR-009, measured).
/// Here every block starts on a [`GRAIN`] boundary of one page-aligned reservation and nothing is
/// reused, so a block's alignment class and an allocation's cost never depend on history.
///
/// The counts it gives measure computation, with allocation at a constant cost; allocations
/// themselves are V-PERF-11's measure (`Counting`). Freed memory is never reused: a bench that
/// allocates more than [`ARENA_BYTES`] fails with an allocation error.
pub struct Arena;

// SAFETY: `alloc` returns either null or a block of at least `layout.size()` bytes aligned to at
// least `layout.align()`, inside one live reservation that is never freed, and never hands out
// overlapping blocks (the next offset only grows, by compare-exchange). `dealloc` does nothing,
// which is sound: a block simply stays reserved. `realloc` is the trait's default (alloc, copy,
// dealloc), which is correct for any sound `alloc`.
unsafe impl GlobalAlloc for Arena {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = arena_base();
        if base.is_null() {
            return ptr::null_mut();
        }
        let align = layout.align().max(GRAIN);
        let Some(size) = layout.size().checked_next_multiple_of(GRAIN) else {
            return ptr::null_mut();
        };
        let mut next = ARENA_NEXT.load(Ordering::Relaxed);
        loop {
            let Some(start) = next.checked_next_multiple_of(align) else {
                return ptr::null_mut();
            };
            let Some(end) = start.checked_add(size).filter(|&e| e <= ARENA_BYTES) else {
                exhausted(layout.size());
            };
            match ARENA_NEXT.compare_exchange_weak(next, end, Ordering::Relaxed, Ordering::Relaxed)
            {
                // SAFETY: `start + size <= ARENA_BYTES`, so the block is inside the reservation.
                Ok(_) => return unsafe { base.add(start) },
                Err(current) => next = current,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

/// Stops the process when the arena is spent: a message naming the request and the reservation,
/// then an abort. The benchmark harness has already printed the bench's name. It writes from a
/// stack buffer to unbuffered stderr, so it allocates nothing.
fn exhausted(request: usize) -> ! {
    use std::io::Write as _;
    let mut digits = [0u8; 20];
    let mut at = digits.len();
    let mut n = request;
    loop {
        at -= 1;
        digits[at] = b'0' + u8::try_from(n % 10).unwrap_or(0);
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let mut err = std::io::stderr().lock();
    let _written = err
        .write_all(b"rh-bench arena exhausted: a request of ")
        .and_then(|()| err.write_all(&digits[at..]))
        .and_then(|()| {
            err.write_all(
                b" bytes does not fit the 1 GiB reservation (ARENA_BYTES, crates/rh-bench/src/\
                  alloc_count.rs); the bench named above outgrew it\n",
            )
        });
    std::process::abort()
}

/// The arena's reservation, made by the first allocation (null if the system refuses it).
fn arena_base() -> *mut u8 {
    let base = ARENA.load(Ordering::Acquire);
    if !base.is_null() {
        return base;
    }
    let Ok(layout) = Layout::from_size_align(ARENA_BYTES, ARENA_ALIGN) else {
        return ptr::null_mut();
    };
    // SAFETY: a valid layout of non-zero size.
    let fresh = unsafe { System.alloc(layout) };
    match ARENA.compare_exchange(ptr::null_mut(), fresh, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => fresh,
        Err(winner) => {
            // SAFETY: `fresh` came from `System.alloc` with `layout` and was never handed out.
            unsafe { System.dealloc(fresh, layout) };
            winner
        }
    }
}
