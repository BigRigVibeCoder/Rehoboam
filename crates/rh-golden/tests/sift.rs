//! `rh-cli sift` (SPR-013 T-11; V-PLN-04 subset; ADR-0024).
//!
//! On `solo`, `household` and `hamlet`: sifting a run twice writes byte-identical channel-C
//! segments with equal canonical digests, one per channel-A segment, and leaves channel A, the
//! keyframes, the journal and the ladder byte-identical (plane C never feeds plane A). Every
//! `DEATH` completes one `life_ends` match, and `tail` shows it as an annotation after the event.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers fail loudly"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rh_golden::ladder::RungKind;
use rh_golden::run::{RunArgs, run};
use rh_golden::sift::{annotations, sift};
use rh_golden::tail::{TailArgs, tail};
use rh_sifter::sift::read_channel_c;
use rh_stream::record::Body;
use rh_stream::writer::read_channel_a;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn files(dir: &Path, skip: &str) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, at: &Path, skip: &str, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(at).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(root).unwrap().display().to_string();
            if rel.contains(skip) {
                continue;
            }
            if p.is_dir() {
                walk(root, &p, skip, out);
            } else {
                out.insert(rel, std::fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, skip, &mut out);
    out
}

#[test]
fn sifting_twice_is_byte_identical_and_changes_nothing_in_plane_a() {
    for (world, years) in [("solo", 80), ("household", 120), ("hamlet", 120)] {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("sift")
            .join(world);
        let _ = std::fs::remove_dir_all(&dir);
        let mut a = RunArgs::new(&repo(), years, 1, RungKind::Yearly, &dir.join("run.ladder"));
        world.clone_into(&mut a.world);
        a.journal = dir.join("journal.sqlite");
        a.run_dir = Some(dir.clone());
        run(&a).unwrap();
        let plane_a = files(&dir, "/C");
        let (_, first) = sift(&repo(), &dir).unwrap();
        let c1 = files(&dir.join("stream/t0000000000/C"), "\u{0}");
        let (_, second) = sift(&repo(), &dir).unwrap();
        let c2 = files(&dir.join("stream/t0000000000/C"), "\u{0}");
        assert_eq!(first, second, "{world}");
        assert_eq!(c1, c2, "{world}: a re-sift changed channel C");
        assert_eq!(
            files(&dir, "/C"),
            plane_a,
            "{world}: sifting changed channel A or the run"
        );
        let segs_a = read_channel_a(&dir.join("stream"), 0, 518_400).unwrap();
        let segs_c = read_channel_c(&dir, 0).unwrap();
        assert_eq!(segs_a.len(), segs_c.len());
        for (sa, sc) in segs_a.iter().zip(&segs_c) {
            assert_eq!(sc.row.a_canonical_digest, sa.row.canonical_digest);
            assert_eq!(sc.row.a_last_seq, sa.row.last_seq);
        }
        let deaths = segs_a
            .iter()
            .flat_map(|s| &s.records)
            .filter(|r| matches!(&r.body, Body::Event(e) if e.verb == "DEATH"))
            .count();
        assert_eq!(
            usize::try_from(first.matches).unwrap(),
            deaths,
            "{world}: one match per death"
        );
        let notes = annotations(&repo(), &dir).unwrap();
        assert_eq!(notes.values().map(Vec::len).sum::<usize>(), deaths);
        let mut out = Vec::new();
        let repo = repo();
        tail(
            &TailArgs {
                run_dir: dir.clone(),
                repo: repo.clone(),
                kinds: vec!["DEATH".to_owned()],
                ..TailArgs::default()
            },
            &|d: &Path| annotations(&repo, d),
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(
            text.matches("^ match solitary_life/life_ends A=").count(),
            deaths
        );
        println!(
            "T-11 V-PLN-04: {world}: two sifts byte-identical, {} matches in {} channel-C segments; channel A, keyframes, journal and ladder unchanged",
            first.matches,
            segs_c.len()
        );
    }
}
