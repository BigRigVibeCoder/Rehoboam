//! Plane-hash properties (SPR-007 T-01): a one-row change alters exactly one chunk; the Merkle
//! cache's dirty tracking equals a full recomputation; planes B and C over empty and synthetic
//! table sets; the name-order rule; the divergence report.
//!
//! Stability across runs and machines is the golden test (`tests/golden.rs`).
//!
//! REF: `CODEX/05_PROJECT/SPR-007_HashesKeyframesJournal.md` T-01 and ruling 1;
//! `docs/05_architecture/05.1_simulation_core.md` §6 item 6.
#![allow(clippy::unwrap_used, clippy::many_single_char_names, reason = "tests")]

use std::collections::BTreeMap;

use rh_core::world::WorldTables;
use rh_snapshot::hash::{
    CHUNK_ROWS, Divergence, HashError, MerkleCache, Plane, PlaneDigest, TableDigest,
    empty_plane_hash, plane_hash, state_hash,
};

/// A deterministic column of `n` rows.
fn column(n: usize, salt: i64) -> Vec<i64> {
    (0..n)
        .map(|i| {
            i64::try_from(i)
                .unwrap()
                .wrapping_mul(0x9E37_79B9)
                .wrapping_add(salt)
        })
        .collect()
}

fn world(rows: usize) -> WorldTables {
    let mut w = WorldTables::new();
    w.insert("ledger.wealth", column(rows, 1));
    w.insert("pop.count", column(rows, 2));
    w.insert("agenda.due", column(rows / 3, 3));
    w
}

#[test]
fn a_one_row_change_alters_exactly_one_chunk() {
    let rows = 5 * CHUNK_ROWS + 17;
    let base = column(rows, 7);
    let before = TableDigest::of("t", base.as_slice());
    assert_eq!(before.chunks.len(), 6);
    for row in [0, CHUNK_ROWS - 1, CHUNK_ROWS, 3 * CHUNK_ROWS + 5, rows - 1] {
        let mut changed = base.clone();
        changed[row] ^= 1;
        let after = TableDigest::of("t", changed.as_slice());
        let differing: Vec<usize> = (0..before.chunks.len())
            .filter(|&k| before.chunks[k] != after.chunks[k])
            .collect();
        assert_eq!(differing, vec![row / CHUNK_ROWS], "row {row}");
        assert_ne!(before.root, after.root);
    }
}

#[test]
fn the_state_hash_sees_every_column_and_ignores_versions() {
    let a = world(3000);
    let mut b = world(3000);
    assert_eq!(state_hash(&a), state_hash(&b));
    // Re-inserting the same rows changes the version, not the state.
    let rows = b.get("pop.count").unwrap().clone();
    b.insert("pop.count", rows);
    assert_ne!(a.version("pop.count"), b.version("pop.count"));
    assert_eq!(state_hash(&a), state_hash(&b));
    // A changed value, a renamed column, a moved value, a dropped row: all change the hash.
    let mut c = world(3000);
    let mut rows = c.get("agenda.due").unwrap().clone();
    rows[999] += 1;
    c.insert("agenda.due", rows);
    assert_ne!(state_hash(&a), state_hash(&c));
    let mut d = world(3000);
    let rows = d.get("agenda.due").unwrap().clone();
    d.insert("agenda.duf", rows);
    assert_ne!(state_hash(&world(3000)), state_hash(&d));
    let mut e = world(3000);
    let mut rows = e.get("pop.count").unwrap().clone();
    rows.pop();
    e.insert("pop.count", rows);
    assert_ne!(state_hash(&a), state_hash(&e));
}

#[test]
fn the_cache_rehashes_only_new_versions_and_matches_a_full_recomputation() {
    let mut w = world(4 * CHUNK_ROWS);
    let mut cache = MerkleCache::new(Plane::A);
    let h0 = cache.update_world(&w);
    assert_eq!(h0, state_hash(&w));
    let first = cache.rehashed_chunks();
    assert_eq!(first, 4 + 4 + 2);
    // Nothing changed: nothing is re-hashed.
    assert_eq!(cache.update_world(&w), h0);
    assert_eq!(cache.rehashed_chunks(), first);
    // One column replaced: only its chunks are re-hashed.
    let mut rows = w.get("ledger.wealth").unwrap().clone();
    rows[10] = -1;
    w.insert("ledger.wealth", rows);
    let h1 = cache.update_world(&w);
    assert_eq!(cache.rehashed_chunks(), first + 4);
    assert_eq!(h1, state_hash(&w));
    assert!(cache.verify(&w));
    // Growth, shrinkage and removal.
    w.insert("pop.count", column(4 * CHUNK_ROWS + 1, 2));
    assert_eq!(cache.update_world(&w), state_hash(&w));
    w.insert("pop.count", column(CHUNK_ROWS - 3, 2));
    assert_eq!(cache.update_world(&w), state_hash(&w));
    let mut smaller = WorldTables::new();
    for name in ["ledger.wealth", "pop.count"] {
        smaller.insert(name, w.get(name).unwrap().clone());
    }
    assert_eq!(cache.update_world(&smaller), state_hash(&smaller));
    assert!(cache.verify(&smaller));
}

#[test]
fn row_marks_rehash_exactly_the_marked_chunks() {
    let mut rows = column(6 * CHUNK_ROWS, 5);
    let mut cache = MerkleCache::new(Plane::A);
    cache.update("t", None, rows.as_slice());
    let base = cache.rehashed_chunks();
    rows[2 * CHUNK_ROWS + 9] = 42;
    cache.mark_rows("t", 2 * CHUNK_ROWS + 9..2 * CHUNK_ROWS + 10);
    let root = cache.update("t", None, rows.as_slice());
    assert_eq!(cache.rehashed_chunks(), base + 1);
    assert_eq!(root, TableDigest::of("t", rows.as_slice()).root);
    // A range across a chunk boundary marks both chunks.
    rows[CHUNK_ROWS - 1] = 1;
    rows[CHUNK_ROWS] = 1;
    cache.mark_rows("t", CHUNK_ROWS - 1..CHUNK_ROWS + 1);
    let root = cache.update("t", None, rows.as_slice());
    assert_eq!(cache.rehashed_chunks(), base + 3);
    assert_eq!(root, TableDigest::of("t", rows.as_slice()).root);
    // Appending rows re-hashes the last old chunk and the new ones only.
    rows.extend(column(CHUNK_ROWS + 1, 9));
    let root = cache.update("t", None, rows.as_slice());
    assert_eq!(cache.rehashed_chunks(), base + 3 + 2);
    assert_eq!(root, TableDigest::of("t", rows.as_slice()).root);
}

#[test]
fn planes_b_and_c_hash_empty_and_synthetic_table_sets() {
    let empty: [(&str, u64); 0] = [];
    for plane in [Plane::A, Plane::B, Plane::C] {
        assert_eq!(plane_hash(plane, empty).unwrap(), empty_plane_hash(plane));
    }
    assert_ne!(empty_plane_hash(Plane::B), empty_plane_hash(Plane::C));
    assert_ne!(empty_plane_hash(Plane::A), empty_plane_hash(Plane::B));
    assert_eq!(state_hash(&WorldTables::new()), empty_plane_hash(Plane::A));

    // Synthetic plane-B tables: a pins table and a facts ledger with variable-length rows.
    let pins: Vec<Vec<u8>> = (0..3000u32).map(|i| i.to_le_bytes().to_vec()).collect();
    let facts: Vec<Vec<u8>> = (0..50u32)
        .map(|i| vec![b'f'; usize::try_from(i).unwrap()])
        .collect();
    let mut tables = BTreeMap::new();
    tables.insert(
        "facts".to_owned(),
        TableDigest::of("facts", facts.as_slice()),
    );
    tables.insert("pins".to_owned(), TableDigest::of("pins", pins.as_slice()));
    let b = PlaneDigest::from_tables(Plane::B, tables.clone());
    let c = PlaneDigest::from_tables(Plane::C, tables.clone());
    assert_ne!(b.hash, c.hash);
    assert_eq!(c.first_divergence(&b), Some(Divergence::Plane));
    let roots = [
        ("facts", tables["facts"].root),
        ("pins", tables["pins"].root),
    ];
    assert_eq!(plane_hash(Plane::B, roots).unwrap(), b.hash);
    // The table name is part of the root: the same rows under two names differ.
    assert_ne!(
        TableDigest::of("pins", pins.as_slice()).root,
        TableDigest::of("pinz", pins.as_slice()).root
    );
    // Row boundaries are part of the encoding: ["ab", ""] and ["a", "b"] differ.
    let x: Vec<Vec<u8>> = vec![b"ab".to_vec(), Vec::new()];
    let y: Vec<Vec<u8>> = vec![b"a".to_vec(), b"b".to_vec()];
    assert_ne!(
        TableDigest::of("t", x.as_slice()).root,
        TableDigest::of("t", y.as_slice()).root
    );
}

#[test]
fn table_names_must_be_in_strictly_increasing_byte_order() {
    let unsorted = [("b", 1), ("a", 2)];
    assert!(matches!(
        plane_hash(Plane::B, unsorted),
        Err(HashError::TableOrder { .. })
    ));
    let duplicated = [("a", 1), ("a", 1)];
    assert!(plane_hash(Plane::C, duplicated).is_err());
    // Byte order, not locale order: "Z" (0x5A) sorts before "a" (0x61).
    assert!(plane_hash(Plane::A, [("Z", 0), ("a", 0)]).is_ok());
}

#[test]
fn the_divergence_report_names_the_first_table_and_chunk() {
    let a = world(3 * CHUNK_ROWS);
    let mut b = world(3 * CHUNK_ROWS);
    assert_eq!(
        PlaneDigest::of_world(&a).first_divergence(&PlaneDigest::of_world(&b)),
        None
    );
    let mut rows = b.get("pop.count").unwrap().clone();
    rows[2 * CHUNK_ROWS + 1] = 0;
    b.insert("pop.count", rows);
    let mut rows = b.get("ledger.wealth").unwrap().clone();
    rows[CHUNK_ROWS] = 0;
    b.insert("ledger.wealth", rows);
    // "agenda.due" < "ledger.wealth" < "pop.count": ledger is the first divergent table.
    assert_eq!(
        PlaneDigest::of_world(&a).first_divergence(&PlaneDigest::of_world(&b)),
        Some(Divergence::Chunk {
            table: "ledger.wealth".to_owned(),
            chunk: 1,
            first_row: CHUNK_ROWS,
        })
    );
    b.insert("extra.col", vec![1]);
    assert_eq!(
        PlaneDigest::of_world(&a).first_divergence(&PlaneDigest::of_world(&b)),
        Some(Divergence::Table {
            table: "extra.col".to_owned()
        })
    );
}

/// VER-SPR-007 DEF-1 (the Tester's repro): two clones that each replace a column give it the same
/// per-instance version; the cache must not mistake one clone for the other.
#[test]
fn p1_stale_cache_across_clones() {
    let mut w1 = world(3000);
    let mut cache = MerkleCache::new(Plane::A);
    cache.update_world(&w1);
    let mut w2 = w1.clone();
    w1.insert("pop.count", column(3000, 100));
    w2.insert("pop.count", column(3000, 200));
    assert_eq!(w1.version("pop.count"), w2.version("pop.count"));
    assert_ne!(w1.identity(), w2.identity());
    assert_eq!(cache.update_world(&w1), state_hash(&w1));
    assert_eq!(cache.update_world(&w2), state_hash(&w2));
    assert_ne!(state_hash(&w1), state_hash(&w2));
    assert!(cache.verify(&w2));
    // And back: the first world again is re-hashed in full, not served from w2's digest.
    assert_eq!(cache.update_world(&w1), state_hash(&w1));
}

/// Identities are bookkeeping: equal tables of different instances hash equal and compare equal.
#[test]
fn the_identity_never_reaches_the_hash_or_equality() {
    let a = world(2000);
    let b = world(2000);
    let c = a.clone();
    assert_ne!(a.identity(), b.identity());
    assert_ne!(a.identity(), c.identity());
    assert_eq!(a, b);
    assert_eq!(a, c);
    assert_eq!(state_hash(&a), state_hash(&b));
    assert_eq!(state_hash(&a), state_hash(&c));
}
