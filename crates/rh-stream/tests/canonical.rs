//! The canonical text form against its reference (`specs/stream/canonical_v0.py`, SPR-013 T-09).
//!
//! - Golden: every line of `specs/stream/golden/canonical_v0.txt` parses and renders back byte
//!   for byte, every block's digest is the reference's, and every `match_id` vector is reproduced.
//! - Property: random records of every kind, over many generator seeds and sizes, round-trip
//!   (`parse(render(r)) = r`, `render(parse(l)) = l`), and the printer is idempotent.
//! - Refusals: the planted non-canonical lines of `canonical_v0.md` §5 are refused, and so are the
//!   planted files of §1.
#![allow(
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::items_after_statements,
    clippy::cast_possible_truncation,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test helpers: a malformed golden file must fail the test loudly"
)]

use rh_stream::canonical::{self, digest, digest_file, match_id, parse, render};
use rh_stream::record::{
    Binding, Body, Cause, ClauseEvent, Event, Header, Input, Keyframe, Kind, LifeEntry, LifeExit,
    Match, Record, Role, StepDigest,
};

const GOLDEN: &str = include_str!("../../../specs/stream/golden/canonical_v0.txt");

fn hex16(s: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

#[test]
fn every_golden_line_round_trips_and_every_digest_and_match_id_is_the_references() {
    let lines: Vec<&str> = GOLDEN.lines().filter(|l| !l.starts_with('#')).collect();
    let (mut i, mut blocks, mut records, mut ids) = (0, 0, 0, 0);
    while i < lines.len() {
        let words: Vec<&str> = lines[i].split(' ').collect();
        match words[0] {
            "segment" | "lines" => {
                let n: usize = words.last().unwrap().parse().unwrap();
                let block = &lines[i + 1..=i + n];
                for line in block {
                    let rec = parse(line).unwrap_or_else(|e| panic!("{line}: {e}"));
                    assert_eq!(render(&rec).unwrap(), *line);
                    assert_eq!(parse(&render(&rec).unwrap()).unwrap(), rec);
                    records += 1;
                }
                let want = lines[i + n + 1].strip_prefix("digest ").unwrap();
                assert_eq!(canonical::hex16(&digest(block)), want, "block at {i}");
                blocks += 1;
                i += n + 2;
            }
            "match_id" => {
                let (pack, pattern) = (
                    canonical::decode_str(words[1]).unwrap(),
                    canonical::decode_str(words[2]).unwrap(),
                );
                let version: u16 = words[3].parse().unwrap();
                let events: Vec<u64> = words[4]
                    .trim_matches(['[', ']'])
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.parse().unwrap())
                    .collect();
                let bindings: Vec<Binding> = words[5]
                    .trim_matches(['[', ']'])
                    .split(',')
                    .filter(|s| !s.is_empty())
                    .map(|b| {
                        let p: Vec<&str> = b.split('/').collect();
                        Binding {
                            role: p[0].to_owned(),
                            entity_kind: p[1].parse().unwrap(),
                            entity_id: p[2].parse().unwrap(),
                        }
                    })
                    .collect();
                let got = match_id(&pack, &pattern, version, &events, &bindings);
                assert_eq!(format!("{got:016x}"), words[6], "{}", lines[i]);
                ids += 1;
                i += 1;
            }
            other => panic!("unknown golden line {other}"),
        }
    }
    assert!(
        blocks >= 5 && records >= 50 && ids >= 6,
        "{blocks} {records} {ids}"
    );
    let empty: [&str; 0] = [];
    assert_eq!(
        canonical::hex16(&digest(&empty)),
        "99aa06d3014798d86001c324468d497f"
    );
    println!(
        "T-09 canonical: {records} golden lines round-trip, {blocks} digests and {ids} match_id \
         vectors equal the reference's"
    );
}

/// `SplitMix64`: the test's own generator (no third-party crate).
struct Gen(u64);

impl Gen {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[usize::try_from(self.below(xs.len() as u64)).unwrap()]
    }
    fn u64(&mut self) -> u64 {
        let r = self.next();
        self.pick(&[0, u64::MAX, 1, r])
    }
    fn i64(&mut self) -> i64 {
        let r = self.next().cast_signed();
        self.pick(&[0, i64::MIN, i64::MAX, -1, r])
    }
    fn u8(&mut self, n: u8) -> u8 {
        u8::try_from(self.below(u64::from(n))).unwrap()
    }
    fn string(&mut self) -> String {
        const POOL: [&str; 12] = [
            "",
            "M2.demography",
            "a b",
            "%",
            "100%",
            "é",
            "日本",
            "tab\there",
            "new\nline",
            "\\",
            "~",
            "[1,2]",
        ];
        if self.below(2) == 0 {
            return self.pick(&POOL).to_owned();
        }
        (0..=self.below(12))
            .filter_map(|_| char::from_u32(u32::try_from(self.below(0x11_0000)).unwrap()))
            .collect()
    }
    fn blob(&mut self) -> [u8; 16] {
        let (a, b) = (self.next(), self.next());
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&a.to_le_bytes());
        out[8..].copy_from_slice(&b.to_le_bytes());
        out
    }
    fn len(&mut self, size: u64) -> usize {
        usize::try_from(self.below(size + 1)).unwrap()
    }
}

fn record(g: &mut Gen, kind: Kind, size: u64) -> Record {
    let header = Header {
        timeline_id: g.pick(&[0, u32::MAX, 7]),
        seq: g.u64(),
        tick: g.i64(),
    };
    let body = match kind {
        Kind::Input => Body::Input(Input {
            journal_seq: g.u64(),
            class: 0,
            input_kind: g.string(),
            scope: g.string(),
            scope_id: (g.below(3) > 0).then(|| g.u64()),
            producer: g.string(),
            payload_hash: g.blob(),
            row_digest: g.blob(),
        }),
        Kind::Event => {
            let mut causes: Vec<u64> = (0..g.len(size)).map(|_| g.u64()).collect();
            causes.sort_unstable();
            causes.dedup();
            Body::Event(Event {
                event_id: g.u64(),
                phase: g.u8(12),
                verb_id: u16::try_from(g.below(65_536)).unwrap(),
                verb: g.string(),
                act_classes: g.u64(),
                place_id: (g.below(2) == 0).then(|| u32::try_from(g.below(1 << 32)).unwrap()),
                magnitude: i32::try_from(g.below(1 << 32).cast_signed() - (1 << 31)).unwrap(),
                tellability: g.u8(255),
                secrecy: g.u8(4),
                origin: g.pick(&[0, 1, 3, 4]),
                payload: (0..g.len(size)).map(|_| g.i64()).collect(),
                roles: (0..g.len(size))
                    .map(|_| Role {
                        entity_kind: g.u8(5),
                        entity_id: g.u64(),
                        role: g.u8(4),
                    })
                    .collect(),
                causes: causes
                    .into_iter()
                    .map(|cause_id| Cause {
                        cause_id,
                        kind: g.u8(9),
                    })
                    .collect(),
            })
        }
        Kind::LifeEntry => Body::LifeEntry(LifeEntry {
            person_id: g.u64(),
            origin: g.u8(3),
            sex: g.u8(2),
            mother_id: g.u64(),
            father_id: g.u64(),
            birth_tick: g.i64(),
        }),
        Kind::LifeExit => {
            let death = g.below(2) == 0;
            Body::LifeExit(LifeExit {
                person_id: g.u64(),
                exit_kind: u8::from(!death),
                death_cause: if death { g.u8(5) + 1 } else { 0 },
                event_id: (death && g.below(2) == 0).then(|| g.u64()),
            })
        }
        Kind::StepDigest => Body::StepDigest(StepDigest {
            living: g.u64(),
            births: u32::try_from(g.below(1 << 32)).unwrap(),
            deaths: u32::try_from(g.below(1 << 32)).unwrap(),
            state_hash: g.u64(),
            observation_hash: g.u64(),
            narrative_hash: g.u64(),
        }),
        Kind::Keyframe => Body::Keyframe(Keyframe {
            keyframe_format: g.string(),
            keyframe_path: g.string(),
            state_hash: g.u64(),
            journal_position: g.u64(),
        }),
        Kind::Match => {
            let n = g.len(size).max(1);
            let mut ids: Vec<u64> = (0..n).map(|_| g.u64()).collect();
            ids.sort_unstable();
            ids.dedup();
            let mut roles: Vec<String> = (0..g.len(size))
                .map(|i| format!("{}{i}", g.pick(&["A", "kin_", "_x"])))
                .collect();
            roles.sort();
            roles.dedup();
            let closed = g.pick(&ids);
            Body::Match(Match {
                match_id: g.u64(),
                pack: g.string(),
                pattern_id: g.string(),
                pattern_version: u16::try_from(g.below(65_536)).unwrap(),
                selection: g.u8(3),
                on_complete: g.u8(3),
                status: 1,
                opened_tick: header.tick.saturating_sub(g.below(1 << 20).cast_signed()),
                closed_by_event_id: closed,
                closed_by_seq: g.u64(),
                bindings: roles
                    .into_iter()
                    .map(|role| Binding {
                        role,
                        entity_kind: g.u8(5),
                        entity_id: g.u64(),
                    })
                    .collect(),
                events: ids
                    .into_iter()
                    .enumerate()
                    .map(|(i, event_id)| ClauseEvent {
                        clause: format!("e{}", i + 1),
                        event_id,
                    })
                    .collect(),
            })
        }
    };
    Record { header, body }
}

const KINDS: [Kind; 7] = [
    Kind::Input,
    Kind::Event,
    Kind::LifeEntry,
    Kind::LifeExit,
    Kind::StepDigest,
    Kind::Keyframe,
    Kind::Match,
];

/// Property: 7 kinds x 12 generator seeds x 4 list sizes x 60 records round-trip.
#[test]
fn random_records_of_every_kind_round_trip_over_seeds_and_sizes() {
    let mut n = 0;
    for seed in 0..12u64 {
        for size in [0, 1, 3, 9] {
            let mut g = Gen(seed.wrapping_mul(0x1234_5678_9ABC_DEF1) ^ size);
            for _ in 0..60 {
                for kind in KINDS {
                    let rec = record(&mut g, kind, size);
                    let line = render(&rec).unwrap_or_else(|e| panic!("{rec:?}: {e}"));
                    assert!(line.is_ascii() && !line.contains(['\n', '\r', '\t']));
                    let back = parse(&line).unwrap_or_else(|e| panic!("{line}: {e}"));
                    assert_eq!(back, rec);
                    assert_eq!(render(&back).unwrap(), line);
                    n += 1;
                }
            }
        }
    }
    println!("T-09 canonical property: {n} records round-trip over 12 seeds and 4 sizes");
}

/// `canonical_v0.md` §5's planted lines: each is refused.
#[test]
fn planted_non_canonical_lines_are_refused() {
    let good = [
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] []",
        "LIFE_EXIT 0 11 216000 13107 1 0 ~",
        "STEP_DIGEST 0 12 216000 0 0 1 0000000000000001 0000000000000002 0000000000000003",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [e1/0]",
        "INPUT 0 0 0 0 0 resample person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
    ];
    for g in good {
        assert_eq!(render(&parse(g).unwrap()).unwrap(), g);
    }
    let bad = [
        " LIFE_EXIT 0 11 216000 13107 1 0 ~",
        "LIFE_EXIT 0 11 216000 13107 1 0 ~ ",
        "LIFE_EXIT 0  11 216000 13107 1 0 ~",
        "LIFE_EXIT 0 11 216000 13107 1 0 ~\r",
        "LIFE_EXIT 0 011 216000 13107 1 0 ~",
        "LIFE_EXIT 0 +11 216000 13107 1 0 ~",
        "LIFE_EXIT 0 11 -0 13107 1 0 ~",
        "LIFE_EXIT 0 18446744073709551616 216000 13107 1 0 ~",
        "LIFE_EXIT 4294967296 11 216000 13107 1 0 ~",
        "LIFE_EXIT 0 000000000000000000011 216000 13107 1 0 ~",
        "LIFE_EXIT 0 11 216000 13107 1 0",
        "LIFE_EXIT 0 11 216000 13107 1 0 ~ ~",
        "LIFE_EXIT ~ 11 216000 13107 1 0 ~",
        "life_exit 0 11 216000 13107 1 0 ~",
        "DEPARTURE 0 11 216000 13107 1 0 ~",
        "STEP_DIGEST 0 12 216000 0 0 1 1 0000000000000002 0000000000000003",
        "STEP_DIGEST 0 12 216000 0 0 1 000000000000000A 0000000000000002 0000000000000003",
        "STEP_DIGEST 0 12 216000 0 0 1 00000000000000001 0000000000000002 0000000000000003",
        "INPUT 0 0 0 0 0 resample person 4369 script 0000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0 resample%2a person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0 resample%2 person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0 resample%FF person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0 resampl%65 person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0 res\tample person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0 résample person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 0  person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "INPUT 0 0 0 0 1 resample person 4369 script 00000000000000000000000000000000 01010101010101010101010101010101",
        "EVENT 0 1 0 0 12 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 4 0 [1] [1/4369/1] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 2 [1] [1/4369/1] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [5/4369/1] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/4] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] [0/9]",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 1 [1/4369/1] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1,] [1/4369/1] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369] []",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] [3/1,2/1]",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1] [1/4369/1] [2/1,2/1]",
        "EVENT 0 1 0 0 4 0 DEATH 32768 ~ 0 128 0 0 [1, 2] [1/4369/1] []",
        "LIFE_EXIT 0 11 216000 13107 0 6 ~",
        "LIFE_EXIT 0 11 216000 13107 0 0 ~",
        "LIFE_EXIT 0 11 216000 13107 1 1 ~",
        "LIFE_EXIT 0 11 216000 13107 1 0 5",
        "LIFE_ENTRY 0 2 0 4369 3 0 0 0 -10368000",
        "LIFE_ENTRY 0 2 0 4369 1 2 0 0 -10368000",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 3 0 1 0 0 1 [A/1/4369] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 3 1 0 0 1 [A/1/4369] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 0 0 0 1 [A/1/4369] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] []",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 5 1 [A/1/4369] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 1 0 1 [A/1/4369] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [B/1/4369,A/1/1] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369,A/1/1] [e1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [e1/0,e1/3]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [e1/3,e2/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [f1/0]",
        "MATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [1A/1/4369] [e1/0]",
        "",
    ];
    for b in bad {
        assert!(parse(b).is_err(), "accepted {b:?}");
    }
    println!("T-09 canonical: {} planted lines refused", bad.len());
}

/// `canonical_v0.md` §1's planted files: each is refused, and a good one digests to its lines'.
#[test]
fn planted_non_canonical_files_are_refused() {
    let header = "#rehoboam/stream/canonical/v0 timeline=0 channel=A registry_version=0.2.0 format=rehoboam/stream/channel-a/v0";
    let a = "LIFE_ENTRY 0 0 0 4369 1 0 0 0 -10368000";
    let b = "LIFE_EXIT 0 1 5 4369 0 1 ~";
    let good = format!("{header}\n# a comment\n{a}\n{b}\n");
    let f = digest_file(good.as_bytes()).unwrap();
    assert_eq!(f.records.len(), 2);
    assert_eq!(f.digest, digest(&[a, b]));
    let bad = [
        format!("{header}\r\n{a}\r\n"),
        format!("{header}\n{a}"),
        format!("{header}\n{a}\r{b}\n"),
        format!("{header}\n{a} é\n"),
        format!("{a}\n{b}\n"),
        format!(
            "{header}\nMATCH 0 0 0 ce0cbf3420904cae solitary_life life_ends 1 0 0 1 0 0 1 [A/1/4369] [e1/0]\n"
        ),
        format!("{header}\n{a}\nLIFE_EXIT 0 2 5 4369 0 1 ~\n"),
        format!("{header}\n{a}\nLIFE_EXIT 1 1 5 4369 0 1 ~\n"),
        format!("{header}\nLIFE_ENTRY 0 0 9 4369 1 0 0 0 -1\nLIFE_EXIT 0 1 5 4369 0 1 ~\n"),
        String::new(),
    ];
    for f in &bad {
        assert!(digest_file(f.as_bytes()).is_err(), "accepted {f:?}");
    }
    assert_eq!(
        canonical::hex16(&hex16("99aa06d3014798d86001c324468d497f")),
        "99aa06d3014798d86001c324468d497f"
    );
}

/// The parser's fuzz tier (nightly; SPR-013 T-09): every golden line and many generated ones,
/// mutated by byte flips, insertions, deletions and token swaps, is either refused or already
/// canonical (`render(parse(l)) == l`): no line parses to a record that prints differently, and
/// nothing panics. The PR tier runs the same check on 1/64 of the mutations.
fn fuzz(rounds: u64) -> (u64, u64) {
    let mut lines: Vec<String> = GOLDEN
        .lines()
        .filter(|l| Kind::from_name(l.split(' ').next().unwrap_or("")).is_some())
        .map(str::to_owned)
        .collect();
    let mut g = Gen(0xF022);
    for _ in 0..200 {
        for kind in KINDS {
            lines.push(render(&record(&mut g, kind, 3)).unwrap());
        }
    }
    const ALPHABET: &[u8] = b" 0123456789abcdefABCDEF-~%[],/_:.e\t\r\nZ";
    let (mut refused, mut accepted) = (0u64, 0u64);
    for round in 0..rounds {
        for line in &lines {
            let mut b = line.as_bytes().to_vec();
            for _ in 0..=g.below(3) {
                let at = usize::try_from(g.below(b.len() as u64 + 1)).unwrap();
                let c = ALPHABET[usize::try_from(g.below(ALPHABET.len() as u64)).unwrap()];
                match g.below(4) {
                    0 if at < b.len() => b[at] = c,
                    1 => b.insert(at, c),
                    2 if at < b.len() => {
                        b.remove(at);
                    }
                    _ => {
                        let words: Vec<&[u8]> = b.split(|&x| x == b' ').collect();
                        if words.len() > 2 {
                            let (i, j) = (
                                usize::try_from(g.below(words.len() as u64)).unwrap(),
                                usize::try_from(g.below(words.len() as u64)).unwrap(),
                            );
                            let mut w: Vec<Vec<u8>> = words.iter().map(|x| x.to_vec()).collect();
                            w.swap(i, j);
                            b = w.join(&b' ');
                        }
                    }
                }
            }
            let Ok(text) = String::from_utf8(b) else {
                refused += 1;
                continue;
            };
            match parse(&text) {
                Ok(rec) => {
                    assert_eq!(
                        render(&rec).unwrap(),
                        text,
                        "round {round}: {line} -> {text}"
                    );
                    accepted += 1;
                }
                Err(_) => refused += 1,
            }
        }
    }
    (refused, accepted)
}

#[test]
fn mutated_lines_are_refused_or_already_canonical() {
    let (refused, accepted) = fuzz(1);
    println!("T-09 canonical fuzz (PR tier): {refused} refused, {accepted} canonical");
}

#[test]
#[ignore = "nightly fuzz tier: 64 rounds of mutations of every golden and generated line"]
fn mutated_lines_are_refused_or_already_canonical_nightly() {
    let (refused, accepted) = fuzz(64);
    println!("T-09 canonical fuzz (nightly): {refused} refused, {accepted} canonical");
}
