//! `Catchup`: the viewer's pure catch-up state machine (`wire_v1.md` §6.3; the reference's
//! `Catchup` in `specs/stream/selftest_wire_v1.py`, whose golden transcript this reproduces line
//! for line).
//!
//! A viewer keeps one number, its cursor (the next seq it needs). It feeds every reply, frame,
//! timer and input to [`Catchup::next`] as an [`Event`] and carries out the [`Action`] it returns:
//! subscribe from the cursor, read sealed segments for `[cursor, s]` when sent behind, back off on
//! a retriable refusal, reconnect and check the run on a closed connection, jump to live. A
//! protocol error moves it to [`State::Failed`], which accepts only `Closed`. The transport's four
//! orderings of REV-SPR-026 R15 are handled: a frame in flight under a jump is delivered; a
//! `stream.behind` (or `stream.end`) in flight under a jump ends it, and the `not_subscribed`
//! answer that follows is expected; a close during a sealed read is remembered and answered when
//! the read ends; a retry timer that fires after a reconnect is stale and ignored.
//!
//! It holds no I/O, so `rh-cli tail --live`, the TUI stub and a Tester's harness share one loop.

use crate::control::EndReason;

/// The first backoff, in polls.
pub const BACKOFF_FIRST: u64 = 1;
/// The longest backoff, in polls.
pub const BACKOFF_CAP: u64 = 64;

/// What the viewer's loop hears.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A subscribe response: `first_seq`.
    Subscribed(u64),
    /// `behind_live_edge` (a subscribe's error) or `stream.behind`: the cursor the host saw (the
    /// viewer's own for an error), `sealed_last_seq` and `live_first_seq`.
    Behind {
        /// The cursor the host names.
        next_seq: u64,
        /// `s`: `None` while nothing is sealed.
        s: Option<u64>,
        /// `w0`.
        w0: u64,
    },
    /// A RECORDS frame.
    Records {
        /// Its `first_seq`.
        first_seq: u64,
        /// Its count.
        count: u64,
    },
    /// A `stream.sealed` notification.
    Sealed,
    /// A `stream.end` notification.
    End(EndReason),
    /// The connection closed.
    Closed,
    /// A sealed read ended: the lines it returned.
    SealedRead {
        /// The first seq read.
        first_seq: u64,
        /// The lines read.
        count: u64,
    },
    /// An error reply other than `behind_live_edge`: its reason token, and `future_seq`'s
    /// `finished`.
    Refused {
        /// The reason token.
        reason: String,
        /// Whether the run is over (`future_seq`).
        finished: bool,
    },
    /// A backoff timer fired.
    RetryElapsed,
    /// A `stream.status` answer after a reconnect: whether it is the same run (`history`).
    Status {
        /// The same history as before.
        same_run: bool,
    },
    /// The viewer's input: jump to live.
    Jump,
    /// An unsubscribe response: `next_seq`.
    Unsubscribed(u64),
}

/// What the viewer's loop does next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// `stream.subscribe` from this seq.
    Subscribe(u64),
    /// `stream.subscribe` from `"live"`.
    SubscribeLive,
    /// Read the sealed segments for seqs `from..=to`, then send `SealedRead`.
    ReadSealed {
        /// The first seq.
        from: u64,
        /// The last seq.
        to: u64,
    },
    /// Hand these records to the viewer (count 0: a subscription began).
    Deliver {
        /// The first seq.
        first_seq: u64,
        /// The count.
        count: u64,
    },
    /// Nothing (with why, or empty).
    Ignore(String),
    /// Wait this many polls, then send `RetryElapsed` (with the refusal's reason).
    Wait(u64, String),
    /// Reconnect and ask `stream.status`, then send `Status`.
    Status,
    /// `stream.unsubscribe`.
    Unsubscribe,
    /// The stream ended: the cursor and the reason.
    Done(u64, EndReason),
    /// A protocol error: close the connection (with why).
    Close(String),
    /// Stop for good (another run, or a cursor past the end of a finished run).
    Stop(String),
}

impl Action {
    /// The golden file's form: `<kind> <a> <b> <detail>` with the detail unescaped.
    #[must_use]
    pub fn golden(&self) -> (&'static str, u64, u64, String) {
        match self {
            Self::Subscribe(c) => ("subscribe", *c, 0, String::new()),
            Self::SubscribeLive => ("subscribe_live", 0, 0, String::new()),
            Self::ReadSealed { from, to } => ("read_sealed", *from, *to, String::new()),
            Self::Deliver { first_seq, count } => ("deliver", *first_seq, *count, String::new()),
            Self::Ignore(d) => ("ignore", 0, 0, d.clone()),
            Self::Wait(n, d) => ("wait", *n, 0, d.clone()),
            Self::Status => ("status", 0, 0, String::new()),
            Self::Unsubscribe => ("unsubscribe", 0, 0, String::new()),
            Self::Done(c, r) => ("done", *c, 0, r.name().to_owned()),
            Self::Close(d) => ("close", 0, 0, d.clone()),
            Self::Stop(d) => ("stop", 0, 0, d.clone()),
        }
    }
}

/// The machine's states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Not started.
    Idle,
    /// Reconnected, waiting for `stream.status`.
    Connecting,
    /// A subscribe is in flight.
    Subscribing,
    /// Subscribed: frames arrive.
    Live,
    /// Reading sealed segments.
    ReadingSealed,
    /// Backing off.
    Retry,
    /// A jump's unsubscribe is in flight: frames still arrive and are delivered.
    Jumping,
    /// A `stream.behind` ended the subscription during a jump: `not_subscribed` is expected.
    JumpEnded,
    /// A protocol error: only `Closed` is accepted.
    Failed,
    /// The stream ended.
    Done,
    /// Another run, or a cursor past the end of a finished run.
    Fatal,
}

/// The catch-up loop of one cursor (module docs).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Catchup {
    /// The next seq the viewer needs.
    pub cursor: u64,
    /// The state.
    pub state: State,
    read_to: u64,
    backoff: u64,
    /// `(from, to)` of a jump to live: the seqs skipped.
    pub skipped: Option<(u64, u64)>,
    /// A connection closed during a sealed read.
    pub reconnect_pending: bool,
}

impl Catchup {
    /// A loop whose next seq is `cursor`.
    #[must_use]
    pub const fn new(cursor: u64) -> Self {
        Self {
            cursor,
            state: State::Idle,
            read_to: 0,
            backoff: BACKOFF_FIRST,
            skipped: None,
            reconnect_pending: false,
        }
    }

    /// The first action: subscribe from the cursor.
    pub const fn start(&mut self) -> Action {
        self.state = State::Subscribing;
        Action::Subscribe(self.cursor)
    }

    /// Feeds one event; returns the action it calls for.
    pub fn next(&mut self, event: Event) -> Action {
        if self.state == State::Failed {
            return if event == Event::Closed {
                self.closed()
            } else {
                Action::Ignore(String::new())
            };
        }
        match self.state {
            State::Done => return Action::Ignore("done".to_owned()),
            State::Fatal => return Action::Ignore("fatal".to_owned()),
            _ => {}
        }
        match event {
            Event::Closed => self.closed(),
            Event::Status { same_run } => self.status(same_run),
            Event::Subscribed(n) => self.subscribed(n),
            Event::Behind { next_seq, s, w0 } => self.behind(next_seq, s, w0),
            Event::Records { first_seq, count } => self.records(first_seq, count),
            Event::SealedRead { first_seq, count } => self.read_done(first_seq, count),
            Event::Refused { reason, finished } => self.refused(&reason, finished),
            Event::RetryElapsed => self.retry_elapsed(),
            Event::Sealed => Action::Ignore(String::new()),
            Event::End(reason) => self.end(reason),
            Event::Jump => self.jump(),
            Event::Unsubscribed(n) => self.unsubscribed(n),
        }
    }

    fn fail(&mut self, detail: String) -> Action {
        self.state = State::Failed;
        Action::Close(detail)
    }

    fn closed(&mut self) -> Action {
        if self.state == State::ReadingSealed {
            self.reconnect_pending = true;
            return Action::Ignore("reconnect pending".to_owned());
        }
        self.state = State::Connecting;
        Action::Status
    }

    fn status(&mut self, same_run: bool) -> Action {
        if self.state != State::Connecting {
            return Action::Ignore(String::new());
        }
        if !same_run {
            self.state = State::Fatal;
            return Action::Stop("the socket serves another run (history differs)".to_owned());
        }
        self.state = State::Subscribing;
        Action::Subscribe(self.cursor)
    }

    fn end(&mut self, reason: EndReason) -> Action {
        if !matches!(self.state, State::Live | State::Jumping) {
            return self.fail("stream.end while not subscribed".to_owned());
        }
        self.state = State::Done;
        Action::Done(self.cursor, reason)
    }

    fn subscribed(&mut self, first_seq: u64) -> Action {
        if self.state == State::Jumping {
            self.skipped = Some((self.cursor, first_seq));
            self.cursor = first_seq;
            self.state = State::Live;
            return Action::Deliver {
                first_seq: self.cursor,
                count: 0,
            };
        }
        if self.state != State::Subscribing {
            return self.fail("a subscribe response while not subscribing".to_owned());
        }
        if first_seq != self.cursor {
            return self.fail(format!("first_seq {first_seq} is not the cursor"));
        }
        self.state = State::Live;
        self.backoff = BACKOFF_FIRST;
        Action::Deliver {
            first_seq: self.cursor,
            count: 0,
        }
    }

    fn behind(&mut self, next_seq: u64, s: Option<u64>, w0: u64) -> Action {
        if self.state == State::Jumping {
            self.state = State::JumpEnded;
            return Action::Ignore("behind during a jump".to_owned());
        }
        if !matches!(self.state, State::Subscribing | State::Live) {
            return self.fail("behind while not subscribed or subscribing".to_owned());
        }
        if self.state == State::Live && next_seq != self.cursor {
            return self.fail(format!(
                "stream.behind.next_seq {next_seq} is not the cursor"
            ));
        }
        let s_plus_1 = s.map_or(0, |s| s + 1);
        if s_plus_1 < w0 {
            let shown = s.map_or_else(|| "-1".to_owned(), |s| s.to_string());
            return self.fail(format!(
                "I1 broken: sealed_last_seq {shown} + 1 < live_first {w0}"
            ));
        }
        let Some(s) = s.filter(|&s| s >= self.cursor) else {
            let shown = s.map_or_else(|| "-1".to_owned(), |s| s.to_string());
            return self.fail(format!("behind with sealed_last_seq {shown} below cursor"));
        };
        self.state = State::ReadingSealed;
        self.read_to = s;
        Action::ReadSealed {
            from: self.cursor,
            to: s,
        }
    }

    fn records(&mut self, first_seq: u64, count: u64) -> Action {
        if !matches!(self.state, State::Live | State::Jumping) {
            return self.fail("records while not subscribed".to_owned());
        }
        if first_seq != self.cursor || count == 0 {
            return self.fail(format!(
                "frame first_seq {first_seq} is not the cursor {}",
                self.cursor
            ));
        }
        self.cursor += count;
        Action::Deliver { first_seq, count }
    }

    fn read_done(&mut self, first_seq: u64, count: u64) -> Action {
        if self.state != State::ReadingSealed || first_seq != self.cursor {
            return self.fail("a sealed read that does not start at the cursor".to_owned());
        }
        if count == 0 || first_seq + count - 1 != self.read_to {
            return self.fail(format!("a sealed read of {count} seqs to {}", self.read_to));
        }
        self.cursor = first_seq + count;
        if self.reconnect_pending {
            self.reconnect_pending = false;
            self.state = State::Connecting;
            return Action::Status;
        }
        self.state = State::Subscribing;
        Action::Subscribe(self.cursor)
    }

    fn refused(&mut self, reason: &str, finished: bool) -> Action {
        if self.state == State::JumpEnded && reason == "not_subscribed" {
            self.state = State::Jumping;
            return Action::SubscribeLive;
        }
        if reason == "future_seq" && finished {
            self.state = State::Fatal;
            return Action::Stop(
                "future_seq on a finished run: the cursor is past the end".to_owned(),
            );
        }
        if !crate::control::Reason::from_name(reason).is_some_and(crate::control::Reason::retriable)
        {
            return self.fail(format!("subscribe refused: {reason}"));
        }
        self.state = State::Retry;
        let delay = self.backoff;
        self.backoff = (self.backoff * 2).min(BACKOFF_CAP);
        Action::Wait(delay, reason.to_owned())
    }

    fn retry_elapsed(&mut self) -> Action {
        if self.state != State::Retry {
            return Action::Ignore("a stale retry timer".to_owned());
        }
        self.state = State::Subscribing;
        Action::Subscribe(self.cursor)
    }

    fn jump(&mut self) -> Action {
        if self.state != State::Live {
            return Action::Ignore("jump while not live".to_owned());
        }
        self.state = State::Jumping;
        Action::Unsubscribe
    }

    fn unsubscribed(&mut self, next_seq: u64) -> Action {
        if !matches!(self.state, State::Jumping | State::JumpEnded) {
            return self.fail("unsubscribed while not jumping".to_owned());
        }
        if self.state == State::Jumping && next_seq != self.cursor {
            return self.fail("unsubscribe.next_seq is not the cursor".to_owned());
        }
        self.state = State::Jumping;
        Action::SubscribeLive
    }
}
