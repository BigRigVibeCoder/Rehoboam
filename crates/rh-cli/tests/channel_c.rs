//! Channel C live (SPR-026 T-04; `specs/stream/wire_v1.md` §6.4, §9 "Channel C live").
//!
//! `hamlet` runs with `--socket`; while only the engine host serves there is no `C.sock`. Then
//! `rh-cli sift D --follow --socket` sifts beside it, and a viewer subscribed on `C.sock` from seq
//! 0 reads to `stream.end` through sim-years with no match:
//! - its lines are `rh-cli tail --channel C --canonical`'s over the sealed segments;
//! - every empty year arrives as `stream.sealed` with `last_seq` null, the year's first seq and the
//!   digest of the empty string, and every non-empty year's digest is the bodies' XXH3-128;
//! - `sifted_through.a_seq` never exceeds channel A's `sealed.last_seq`;
//! - channel C equals a batch `rh-cli sift` of the same run, byte for byte, and the state ladder
//!   is the baseline (V-PLN-04).
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a tool-crate test that runs rh-cli as child processes, one probe in one place; \
              helpers fail loudly"
)]

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use rh_protocol::control::Kind;
use serde_json::json;
use support::{BIN, Client, Host, In, a_sock, baseline, c_sock, ladders_equal, scratch};

/// The digest of the empty string (`golden/wire_v1.txt` `notify_sealed_empty`).
const EMPTY_DIGEST: &str = "99aa06d3014798d86001c324468d497f";

fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, at: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(at)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().display().to_string(),
                    std::fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// The canonical digest of a body: XXH3-128 of its bytes (`canonical_v0.md` §4).
fn xxh3_hex(body: &[u8]) -> String {
    rh_snapshot::hash::xxh3_128(body)
        .iter()
        .fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            let _ = write!(s, "{b:02x}");
            s
        })
}

#[test]
fn channel_c_is_served_live_beside_the_run_with_its_empty_years() {
    let dir = scratch("chan-c");
    let d = dir.display().to_string();
    let run = Host::start(&[
        "run",
        "hamlet",
        "--years",
        "200",
        "--run-dir",
        &d,
        "--socket",
        "--linger",
        "60",
    ]);
    support::wait_socket(&a_sock(&dir));
    assert!(
        !c_sock(&dir).exists(),
        "the engine process never serves channel C"
    );
    let mut a = Client::connect(&a_sock(&dir));
    let sift = Host::start(&["sift", &d, "--follow", "--socket", "--linger", "60"]);
    let mut c = Client::connect(&c_sock(&dir));
    let (reply, _) = c.call("stream.subscribe", Some(json!({"from": {"seq": 0}})));
    assert_eq!(reply["result"]["first_seq"], 0, "{reply}");
    let mut status = Client::connect(&c_sock(&dir));
    let mut lines: Vec<String> = Vec::new();
    let mut bodies: BTreeMap<i64, Vec<u8>> = BTreeMap::new();
    let mut year_of: Vec<(u64, i64)> = Vec::new(); // (first seq, year) of announced years
    let (mut empty, mut full, mut checks) = (0, 0, 0);
    let mut pending_body: Vec<(u64, Vec<u8>)> = Vec::new();
    loop {
        match c.next().expect("the sifter closed before stream.end") {
            In::Records(r, body) => {
                let mut at = 0;
                for (i, l) in r.lines.iter().enumerate() {
                    let n = l.len() + 1;
                    pending_body.push((
                        r.first_seq + u64::try_from(i).unwrap(),
                        body[at..at + n].to_vec(),
                    ));
                    at += n;
                    lines.push(l.clone());
                }
            }
            In::Control(Kind::Notification, v) if v["method"] == "stream.sealed" => {
                let p = &v["params"];
                assert_eq!(p["channel"], "C");
                let year = p["sim_year"].as_i64().unwrap();
                let first = p["first_seq"].as_u64().unwrap();
                if p["last_seq"].is_null() {
                    assert_eq!(p["canonical_digest"], EMPTY_DIGEST, "year {year}");
                    assert_eq!(
                        first,
                        u64::try_from(lines.len()).unwrap(),
                        "an empty year's first seq"
                    );
                    empty += 1;
                } else {
                    let last = p["last_seq"].as_u64().unwrap();
                    let body: Vec<u8> = pending_body
                        .iter()
                        .filter(|(s, _)| (first..=last).contains(s))
                        .flat_map(|(_, b)| b.clone())
                        .collect();
                    assert_eq!(xxh3_hex(&body), p["canonical_digest"], "year {year}");
                    bodies.insert(year, body);
                    full += 1;
                }
                year_of.push((first, year));
                // sifted_through never exceeds channel A's sealed.last_seq (A's s only grows, so
                // reading A after C is a fair comparison).
                if checks < 40 {
                    let (cs, _) = status.call("stream.status", None);
                    let (as_, _) = a.call("stream.status", None);
                    let sifted = cs["result"]["sifted_through"]["a_seq"].as_u64();
                    let a_last = as_["result"]["sealed"]["last_seq"].as_u64();
                    assert!(sifted.is_some(), "{cs}");
                    assert!(
                        sifted <= a_last,
                        "sifted_through {sifted:?} > A's sealed {a_last:?}"
                    );
                    checks += 1;
                }
            }
            In::Control(Kind::Notification, v) if v["method"] == "stream.end" => {
                assert_eq!(v["params"]["reason"], "finished");
                break;
            }
            In::Control(Kind::Notification, v) => panic!("{v}"),
            In::Control(..) => {}
        }
    }
    assert!(
        empty > 0 && full > 0,
        "{empty} empty and {full} non-empty years"
    );
    assert_eq!(empty + full, 200, "every sim-year announced");
    drop((a, c, status));
    let (code, out, err) = sift.finish();
    assert_eq!(code, 0, "{out:?} {err}");
    let (code, out, err) = run.finish();
    assert_eq!(code, 0, "{out:?} {err}");
    // The viewer's lines are tail's over the sealed channel C.
    let tail = Command::new(BIN)
        .args(["tail", &d, "--channel", "C", "--canonical"])
        .output()
        .unwrap();
    let sealed: Vec<String> = String::from_utf8(tail.stdout)
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(str::to_owned)
        .collect();
    assert_eq!(lines, sealed, "the live lines are the sealed ones");
    // Following sealed exactly what a batch sift of the same run seals.
    let followed = files(&dir.join("stream/t0000000000/C"));
    let copy = dir.with_extension("batch");
    let cp = Command::new("cp")
        .arg("-a")
        .arg(&dir)
        .arg(&copy)
        .status()
        .unwrap();
    assert!(cp.success());
    let out = Command::new(BIN).arg("sift").arg(&copy).output().unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        files(&copy.join("stream/t0000000000/C")),
        followed,
        "follow = batch, byte for byte"
    );
    assert!(
        ladders_equal(&dir.join("run.ladder"), &baseline("hamlet")),
        "V-PLN-04"
    );
}
