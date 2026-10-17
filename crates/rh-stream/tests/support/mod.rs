//! Synthetic channel-A streams for the segment and writer tests: a world of `persons` founders
//! whose deaths come at keyed instants, with monthly digests, over `years` sim-years. Varying the
//! seed and the size varies every table's row count (GOV-002 lesson: vary world size and seed).
#![allow(
    dead_code,
    clippy::too_many_lines,
    clippy::explicit_counter_loop,
    reason = "test helpers: each test binary uses some; one synthetic history, built in one place"
)]

use std::path::PathBuf;

use rh_stream::record::{
    Body, Cause, Event, Input, Keyframe, LifeEntry, LifeExit, Role, StepDigest,
};
use rh_stream::segment::{Dictionaries, Provenance, VerbEntry};

/// A 360-day calendar's minutes per year.
pub const MPY: i64 = 518_400;
/// A month of that calendar.
pub const MONTH: i64 = 43_200;

/// The dictionaries of the synthetic world: `DEATH` and `EXTINCTION`, as the Registry has them.
#[must_use]
pub fn dictionaries() -> Dictionaries {
    Dictionaries {
        registry_version: "0.2.0".to_owned(),
        verbs: vec![
            VerbEntry {
                verb_id: 0,
                name: "DEATH".to_owned(),
                act_classes: 1,
                tellability: 128,
                emitted_by: "M2.demography".to_owned(),
                aggregate: false,
            },
            VerbEntry {
                verb_id: 1,
                name: "EXTINCTION".to_owned(),
                act_classes: 1,
                tellability: 255,
                emitted_by: "M2.demography".to_owned(),
                aggregate: false,
            },
        ],
        act_classes: vec!["death".to_owned()],
    }
}

/// The synthetic world's history tuple.
#[must_use]
pub fn provenance() -> Provenance {
    Provenance {
        engine_build: "rh-stream-test".to_owned(),
        manifest_hash: vec![0xAB; 32],
        world_seed: 7,
        built_systems: vec!["M2.demography".to_owned()],
        not_built: vec!["M1.regions".to_owned(), "S5.daily_life".to_owned()],
    }
}

fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The `(tick, body)` records of a synthetic run, in no particular order within an instant: the
/// founders' entries at tick 0, one input, each death (a `DEATH` event and its `LIFE_EXIT`, the
/// last one followed by `EXTINCTION`), a digest at every month start and a keyframe marker at
/// each year's last minute. `event_id`s count in emission order.
#[must_use]
pub fn history(seed: u64, persons: u64, years: i64) -> Vec<(i64, Body)> {
    let mut out = Vec::new();
    let horizon = years * MPY;
    let mut deaths: Vec<(i64, u64)> = (0..persons)
        .map(|k| {
            let id = mix(seed ^ k.wrapping_mul(0x1000_0001)) | 1;
            let day =
                i64::try_from(mix(id) % u64::try_from(horizon / 1440).unwrap_or(1)).unwrap_or(0);
            (day * 1440, id)
        })
        .collect();
    deaths.sort_unstable();
    for &(_, id) in &deaths {
        out.push((
            0,
            Body::LifeEntry(LifeEntry {
                person_id: id,
                origin: 1,
                sex: u8::from(id % 2 == 1),
                mother_id: 0,
                father_id: 0,
                birth_tick: -i64::try_from(id % 30_000_000).unwrap_or(0),
            }),
        ));
    }
    out.push((
        7,
        Body::Input(Input {
            journal_seq: 0,
            class: 0,
            input_kind: "oracle_move".to_owned(),
            scope: "person".to_owned(),
            scope_id: deaths.first().map(|d| d.1),
            producer: "system".to_owned(),
            payload_hash: [3; 16],
            row_digest: [4; 16],
        }),
    ));
    let mut event_id = 0;
    let last = deaths.len().saturating_sub(1);
    for (i, &(tick, id)) in deaths.iter().enumerate() {
        out.push((
            tick,
            Body::Event(Event {
                event_id,
                phase: 4,
                verb_id: 0,
                verb: "DEATH".to_owned(),
                act_classes: 1,
                place_id: None,
                magnitude: 0,
                tellability: 128,
                secrecy: 0,
                origin: 0,
                payload: vec![1],
                roles: vec![Role {
                    entity_kind: 1,
                    entity_id: id,
                    role: 1,
                }],
                causes: Vec::new(),
            }),
        ));
        out.push((
            tick,
            Body::LifeExit(LifeExit {
                person_id: id,
                exit_kind: 0,
                death_cause: 1,
                event_id: None,
            }),
        ));
        if i == last {
            out.push((
                tick,
                Body::Event(Event {
                    event_id: event_id + 1,
                    phase: 4,
                    verb_id: 1,
                    verb: "EXTINCTION".to_owned(),
                    act_classes: 1,
                    place_id: None,
                    magnitude: 0,
                    tellability: 255,
                    secrecy: 0,
                    origin: 0,
                    payload: Vec::new(),
                    roles: Vec::new(),
                    causes: vec![Cause {
                        cause_id: event_id,
                        kind: 1,
                    }],
                }),
            ));
        }
        event_id += 1;
    }
    let mut t = 0;
    while t < horizon {
        let living = deaths.iter().filter(|(d, _)| *d > t).count();
        out.push((
            t,
            Body::StepDigest(StepDigest {
                living: u64::try_from(living).unwrap_or(0),
                births: 0,
                deaths: 0,
                state_hash: mix(u64::try_from(t).unwrap_or(0) ^ seed),
                observation_hash: 0x11,
                narrative_hash: 0x22,
            }),
        ));
        t += MONTH;
    }
    for y in 1..=years {
        out.push((
            y * MPY - 1,
            Body::Keyframe(Keyframe {
                keyframe_format: "rehoboam/rh-snapshot/keyframe/v1".to_owned(),
                keyframe_path: format!("keyframes/year-{y:04}.rhkf"),
                state_hash: mix(u64::try_from(y).unwrap_or(0)),
                journal_position: 1,
            }),
        ));
    }
    out
}

/// A fresh directory under Cargo's per-target temporary directory.
#[must_use]
pub fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("rh-stream")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap_or_default();
    dir
}
