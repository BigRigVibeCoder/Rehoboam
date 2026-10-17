//! The method table of §4.5: exactly `stream.status`, `stream.subscribe` and
//! `stream.unsubscribe`, each a read; everything else is `method_not_found` with the read-only
//! message (SPR-026 T-01). The planted control `mutant-pace-method` adds `stream.pace` and must
//! fail here, naming the extra entry.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::cast_possible_truncation,
    clippy::format_collect,
    clippy::items_after_statements,
    reason = "test helpers fail loudly; generators truncate on purpose"
)]

use std::collections::BTreeSet;

use rh_protocol::control::{
    self, Access, Connection, History, HostView, METHOD_TABLE, READ_ONLY_MESSAGE, Source,
};
use rh_protocol::records::Channel;
use serde_json::{Value, json};

struct Host;

impl Source for Host {
    fn view(&self) -> HostView {
        HostView {
            channel: Channel::A,
            timeline_id: 0,
            run_nonce: 7,
            record_format: "rehoboam/stream/channel-a/v0".to_owned(),
            history: History {
                engine_build: "b".to_owned(),
                registry_version: "0.0.0".to_owned(),
                manifest_hash: "00".repeat(16),
                world_seed: 1,
            },
            minutes_per_year: 518_400,
            tip_tick: Some(5),
            next_seq: 10,
            live_first_seq: 0,
            sealed_last_year: None,
            sealed_last_seq: None,
            pace: Some(("max".to_owned(), "running".to_owned())),
            sifted_through: None,
            finished: false,
            seal_log_len: 0,
        }
    }
}

fn ask(method: &str, params: Option<Value>) -> Value {
    let frame =
        control::control_frame(&control::request(&json!(1), method, params)).unwrap_or_default();
    let mut conn = Connection::default();
    let reply = control::dispatch(&mut conn, &Host, &frame);
    let (f, _) = rh_protocol::wire::decode(&reply.frames.concat(), rh_protocol::wire::MAX_PAYLOAD)
        .unwrap_or_else(|e| panic!("{method}: {e}"));
    control::decode_control(&f.payload)
        .unwrap_or_else(|e| panic!("{method}: {e}"))
        .1
}

#[test]
fn the_method_table_is_exactly_the_three_reads() {
    let names: BTreeSet<&str> = METHOD_TABLE.iter().map(|(n, _, _)| *n).collect();
    let want: BTreeSet<&str> = ["stream.status", "stream.subscribe", "stream.unsubscribe"]
        .into_iter()
        .collect();
    let extra: Vec<&&str> = names.difference(&want).collect();
    assert!(
        extra.is_empty() && names == want,
        "method table broken: the table is {names:?}; extra entries {extra:?} (wire_v1.md §4.5: a \
         stream socket accepts exactly the stream.* reads)"
    );
    println!("method table: exactly stream.status, stream.subscribe and stream.unsubscribe");
    for (name, _, access) in METHOD_TABLE {
        assert_eq!(
            *access,
            Access::Read,
            "method table broken: {name} is not a read"
        );
    }
}

#[test]
fn every_method_outside_the_table_is_the_read_only_refusal() {
    for method in [
        "player.act",
        "timeline.fork",
        "timeline.run",
        "oracle.whisper",
        "stream.pace",
        "console.say",
        "world.seek",
        "stream.Status",
        "",
    ] {
        let reply = ask(method, Some(json!({"x": 1})));
        assert_eq!(
            control::reply_reason(&reply),
            Some("method_not_found"),
            "method table broken: {method:?} was answered {reply}"
        );
        assert_eq!(reply["error"]["message"], READ_ONLY_MESSAGE, "{method}");
    }
    for method in ["stream.status", "stream.unsubscribe"] {
        assert_ne!(
            control::reply_reason(&ask(method, None)),
            Some("method_not_found"),
            "{method}"
        );
    }
}
