//! SPR-020 T-02: `Event::digest` v2 and `EventLog::hash` reproduce the independent stdlib
//! reference (`bin/gen_hash_golden.py`) bit for bit, and a one-field mutation of each field kind
//! changes the digest.
//!
//! The golden file is `tests/golden/event_digest_v2.txt`, written by the reference; never edited
//! by hand. Each `event` line is parsed into an `Event` and, as a draft, appended to one log in
//! order with its origin, so the log path (resolution, sorting, the running fold) is covered too.
#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "a golden test: a malformed line must fail loudly"
)]

use rh_events::{
    CausalEdge, Edge, EdgeKind, Event, EventDraft, EventLog, EventRef, Origin, RoleId, RoleRow,
    Secrecy, Snapshot,
};
use rh_ids::{EntityKind, EntityRef, EventId, PlaceId};

const GOLDEN: &str = include_str!("golden/event_digest_v2.txt");

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_owned().into_boxed_str())
}

fn int<T: std::str::FromStr>(s: &str) -> T {
    s.parse().unwrap_or_else(|_| panic!("not an integer: {s}"))
}

fn list(s: &str) -> Vec<&str> {
    if s == "-" {
        Vec::new()
    } else {
        s.split(',').collect()
    }
}

fn entity(s: &str) -> EntityRef {
    let (kind, raw) = s.split_once(':').expect("kind:raw");
    let kind = EntityKind::ALL
        .into_iter()
        .find(|k| k.name() == kind)
        .expect("an entity kind");
    EntityRef::new(kind, int(raw))
}

fn opt(s: &str) -> Option<&str> {
    (s != "-").then_some(s)
}

/// An `event` line: the event and its digest.
fn parse(line: &str) -> (Event, u64) {
    let f: Vec<&str> = line.split(' ').collect();
    assert_eq!(f.len(), 15, "{line}");
    let roles = list(f[10])
        .into_iter()
        .map(|r| {
            let (role, e) = r.split_once('/').expect("role/entity");
            RoleRow {
                role: RoleId(int(role)),
                entity: entity(e),
            }
        })
        .collect();
    let snapshots = list(f[11])
        .into_iter()
        .map(|s| {
            let p: Vec<&str> = s.split('/').collect();
            Snapshot {
                attr: leak(p[0]),
                entity: entity(p[1]),
                partner: opt(p[2]).map(entity),
                value: int(p[3]),
            }
        })
        .collect();
    let causes = list(f[12])
        .into_iter()
        .map(|c| {
            let p: Vec<&str> = c.split('/').collect();
            Edge {
                cause: EventId::from_raw(int(p[0])),
                kind: EdgeKind::from_code(int(p[1])).expect("an edge kind"),
                motive: opt(p[2]).map(leak),
                weight: int(p[3]),
                evidence: int(p[4]),
            }
        })
        .collect();
    let event = Event {
        id: EventId::from_raw(int(f[1])),
        tick: int(f[2]),
        phase: int(f[3]),
        emitter: leak(f[4]),
        kind: leak(f[5]),
        place: opt(f[6]).map(|p| PlaceId::from_raw(int(p))),
        magnitude: int(f[7]),
        secrecy: Secrecy::from_code(int(f[8])).expect("a secrecy"),
        origin: Origin::from_code(int(f[9])).expect("an origin"),
        admitted: true,
        roles,
        snapshots,
        causes,
        payload: list(f[13]).into_iter().map(int).collect(),
    };
    let digest = u64::from_str_radix(f[14], 16).expect("a hex digest");
    (event, digest)
}

fn draft_of(e: &Event) -> EventDraft {
    let mut d = EventDraft::new(e.kind)
        .magnitude(e.magnitude)
        .secrecy(e.secrecy);
    if let Some(p) = e.place {
        d = d.at(p);
    }
    d.roles.clone_from(&e.roles);
    d.snapshots.clone_from(&e.snapshots);
    // Reversed: append sorts the edges by cause id.
    d.causes = e
        .causes
        .iter()
        .rev()
        .map(|c| CausalEdge {
            cause: EventRef::Known(c.cause),
            kind: c.kind,
            motive: c.motive,
            weight: c.weight,
            evidence: c.evidence,
        })
        .collect();
    d.with_payload(e.payload.clone())
}

/// One-field mutations of each field kind (the reference's `_mutations`, mirrored).
fn mutations(e: &Event) -> Vec<(&'static str, Event)> {
    let mut out: Vec<(&'static str, Event)> = Vec::new();
    let mut m = |name: &'static str, f: &dyn Fn(&mut Event)| {
        let mut x = e.clone();
        f(&mut x);
        out.push((name, x));
    };
    m("id", &|x| x.id = EventId::from_raw(x.id.to_raw() + 1));
    m("tick", &|x| x.tick += 1);
    m("phase", &|x| x.phase ^= 1);
    m("emitter", &|x| x.emitter = leak(&format!("{}x", x.emitter)));
    m("kind", &|x| x.kind = leak(&format!("{}X", x.kind)));
    m("place", &|x| {
        x.place = if x.place.is_some() {
            None
        } else {
            Some(PlaceId::from_raw(0))
        };
    });
    m("magnitude", &|x| x.magnitude ^= 1);
    m("secrecy", &|x| {
        x.secrecy = Secrecy::from_code((x.secrecy.code() + 1) % 4).expect("a secrecy");
    });
    m("origin", &|x| {
        x.origin = if x.origin == Origin::Sim {
            Origin::Player
        } else {
            Origin::Sim
        };
    });
    m("payload", &|x| x.payload.push(0));
    if !e.roles.is_empty() {
        m("role", &|x| {
            x.roles[0].role = RoleId((x.roles[0].role.0 + 1) % 4);
        });
    }
    if !e.snapshots.is_empty() {
        m("snapshot value", &|x| x.snapshots[0].value ^= 1);
        m("snapshot attr", &|x| {
            x.snapshots[0].attr = leak(&format!("{}x", x.snapshots[0].attr));
        });
        m("snapshot partner", &|x| {
            x.snapshots[0].partner = match x.snapshots[0].partner {
                Some(_) => None,
                None => Some(EntityRef::WORLD),
            };
        });
    }
    if !e.causes.is_empty() {
        m("edge kind", &|x| {
            x.causes[0].kind =
                EdgeKind::from_code((x.causes[0].kind.code() + 1) % 6).expect("a kind");
        });
        m("edge weight", &|x| x.causes[0].weight ^= 1);
        m("edge evidence", &|x| x.causes[0].evidence ^= 1);
        m("edge motive", &|x| {
            x.causes[0].motive = match x.causes[0].motive {
                Some(_) => None,
                None => Some("FEAR_OF"),
            };
        });
    }
    out
}

#[test]
fn digest_v2_and_the_log_hash_match_the_reference() {
    let mut log = EventLog::new();
    let mut events = 0_usize;
    let mut log_line = None;
    for line in GOLDEN.lines().filter(|l| !l.starts_with('#')) {
        if let Some(rest) = line.strip_prefix("log ") {
            log_line = Some(rest.to_owned());
            continue;
        }
        let (event, want) = parse(line);
        assert_eq!(event.digest(), want, "digest of event {}", event.id);
        let mut not_admitted = event.clone();
        not_admitted.admitted = false;
        assert_eq!(
            not_admitted.digest(),
            want,
            "the admission flag is not hashed"
        );
        for (name, mutated) in mutations(&event) {
            assert_ne!(mutated.digest(), want, "event {}: mutated {name}", event.id);
        }
        let id = log
            .append(
                event.tick,
                event.phase,
                event.emitter,
                event.origin,
                draft_of(&event),
            )
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(id, event.id);
        assert_eq!(
            log.events().last(),
            Some(&event),
            "the appended event is the golden one"
        );
        events += 1;
    }
    let log_line = log_line.expect("a log line");
    let (n, hash) = log_line.split_once(' ').expect("log <n> <hash>");
    assert_eq!(int::<usize>(n), events);
    assert_eq!(
        log.hash(),
        u64::from_str_radix(hash, 16).expect("hex"),
        "EventLog::hash"
    );
    assert!(events >= 6, "the golden file holds the cases");
}
