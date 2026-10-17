//! Lifecycle command queues, owned by `rh-core` (SPR-006 T-03; `05.1` §2 rule 2).
//!
//! A system that needs a change to a column it does not own submits a typed [`Command`] during
//! its step ([`crate::system::StepCtx::submit`]). The engine merges every step's commands at the
//! end of each scheduler level in the canonical order (SPR-006 ruling 1) and queues each one for
//! the system that applies it: the command's owner, declared once in `AccessSpec::applies` and
//! checked at registration (ruling 5).
//!
//! The owner receives its queue, in `(tick, phase, PersonId, seq)` order, the next time it runs
//! ([`crate::system::StepCtx::take_commands`]): the `(phase, PersonId)` order of `05.1` §2 rule 2,
//! led by the instant the command was submitted at (a queue can hold commands from earlier
//! instants) and closed by `seq`, the merge order, so keys are unique. No other system ever sees
//! the commands, and nothing else can apply them: the queue type has no public way to take a
//! command out.
//!
//! A command is typed by its Registry name and carries the person it concerns and an integer
//! argument list; per-command payload types come with the contract crates (`rh-contracts`).
//!
//! REF: `docs/05_architecture/05.1_simulation_core.md` §2 rules 1–3;
//! `CODEX/05_PROJECT/SPR-006_AgendaEventsInvariants.md` T-03, rulings 1 and 5.

use std::collections::BTreeMap;

use rh_events::{DraftRef, EventRef};
use rh_ids::{EventId, PersonId};

use crate::phase::Phase;
use crate::system::PulseBinding;
use crate::time::SimTime;

/// One payload field of a command as a system submits it: an integer, or an event-typed slot
/// (`05.1` §2 rule 2, "Event-typed slots"). The owner's spec types each field
/// ([`ArgType`]); `StepCtx::submit` refuses an `Int` in an `EventRef` field, an `Event` anywhere
/// else, and an integer outside its field's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Arg {
    /// An integer field: an id's bits, a time, a code, a fixed-point value's raw bits.
    Int(i64),
    /// An `EventRef` field; the merge resolves a `Draft` to its id.
    Event(EventRef),
}

impl From<i64> for Arg {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<EventRef> for Arg {
    fn from(r: EventRef) -> Self {
        Self::Event(r)
    }
}

impl Arg {
    /// The integer, if it is an integer field.
    #[must_use]
    pub const fn int(self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(v),
            Self::Event(_) => None,
        }
    }

    /// The event slot, if it is one.
    #[must_use]
    pub const fn event(self) -> Option<EventRef> {
        match self {
            Self::Event(r) => Some(r),
            Self::Int(_) => None,
        }
    }

    /// The field as its hashed row stores it (`rh-core/engine.commands`, `05.1` v1.14): an
    /// integer as itself, an event slot as its resolved id, or -1 for none. An unresolved `Draft`
    /// never reaches a queue.
    #[must_use]
    pub fn encoded(self) -> i64 {
        match self {
            Self::Int(v) => v,
            Self::Event(r) => r.known().map_or(-1, |id| id.to_raw().cast_signed()),
        }
    }
}

/// A payload field's type, from the closed vocabulary of `[[applies]] payload` (EVO-001 R5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArgType {
    /// A `PersonId` (any `u64` bits).
    Person,
    /// An event-typed slot.
    Event,
    /// A `SimTime` (any `i64` minutes).
    Time,
    /// Any other typed id (`TitleId`, `HouseId`, ...): any `u64` bits.
    Id,
    /// `u8`, `u16`, `u32`: `0..=max`.
    Unsigned(u32),
    /// `u64`: any bits.
    U64,
    /// `i64`.
    I64,
    /// `bool`: 0 or 1.
    Bool,
    /// `q8_8` raw bits: an `i16`.
    Q8_8,
    /// `q16_16` raw bits: an `i32`.
    Q16_16,
    /// `enum(<column>)`: a code below the count, or any code `>= 0` when the count is unknown.
    Enum(Option<usize>),
}

impl ArgType {
    /// The type a spec spells `ty`; `enum_codes` counts the codes of `enum(<column>)` columns.
    #[must_use]
    pub fn parse(ty: &str, enum_codes: &BTreeMap<String, usize>) -> Option<Self> {
        Some(match ty {
            "PersonId" => Self::Person,
            "EventRef" => Self::Event,
            "SimTime" => Self::Time,
            "TitleId" | "OfficeId" | "ClaimId" | "HouseId" | "VowId" | "ArtifactId"
            | "HouseholdId" | "PlaceId" => Self::Id,
            "u8" => Self::Unsigned(8),
            "u16" => Self::Unsigned(16),
            "u32" => Self::Unsigned(32),
            "u64" => Self::U64,
            "i64" => Self::I64,
            "bool" => Self::Bool,
            "q8_8" => Self::Q8_8,
            "q16_16" => Self::Q16_16,
            _ => {
                let column = ty.strip_prefix("enum(")?.strip_suffix(')')?;
                Self::Enum(enum_codes.get(column).copied())
            }
        })
    }

    /// Whether `v` is a value of this (integer) type.
    #[must_use]
    pub fn admits(self, v: i64) -> bool {
        match self {
            Self::Person | Self::Time | Self::Id | Self::U64 | Self::I64 => true,
            Self::Event => false,
            Self::Unsigned(bits) => (0..1_i64 << bits).contains(&v),
            Self::Bool => v == 0 || v == 1,
            Self::Q8_8 => i16::try_from(v).is_ok(),
            Self::Q16_16 => i32::try_from(v).is_ok(),
            Self::Enum(count) => {
                v >= 0 && count.is_none_or(|n| usize::try_from(v).is_ok_and(|v| v < n))
            }
        }
    }
}

/// Whether a command's key person must be living when its draining binding takes it
/// (`[[applies]] key`; the dead-person drop, BCK-002 A-30).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyRule {
    /// `living`: in a world with a ledger, `rh-core` drops it if its key person is dead.
    Living,
    /// `any`: never dropped by `rh-core`.
    Any,
}

/// What registration knows about one registered command (`05.1` §2 rule 2): its owner and
/// draining binding from the owner's `AccessSpec`, its typing from the owner's spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandDecl {
    /// The owner.
    pub owner: &'static str,
    /// The binding that drains it.
    pub drained_by: PulseBinding,
    /// The key rule.
    pub key: KeyRule,
    /// The payload field that repeats the key person, or `None` when it is `Command::person`
    /// itself (`key_field = "person"`).
    pub key_field: Option<usize>,
    /// The typed payload, `(field, type)`; `None` when untyped (a draft spec, or a test stub).
    pub payload: Option<Vec<(String, ArgType)>>,
    /// Whether it carries a sampled outcome (EVO-001 R9).
    pub outcome: bool,
}

impl CommandDecl {
    /// A declaration with no typing: `key = "any"`, untyped payload.
    #[must_use]
    pub const fn untyped(owner: &'static str, drained_by: PulseBinding) -> Self {
        Self {
            owner,
            drained_by,
            key: KeyRule::Any,
            key_field: None,
            payload: None,
            outcome: false,
        }
    }

    /// Whether field `i` is an event-typed slot.
    #[must_use]
    pub fn is_event_field(&self, i: usize) -> bool {
        self.payload
            .as_ref()
            .and_then(|p| p.get(i))
            .is_some_and(|(_, t)| *t == ArgType::Event)
    }
}

/// A lifecycle command as a system submits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The command's Registry name (`[[commands]] command`, e.g. `TieDelta`).
    pub kind: &'static str,
    /// The person it concerns: the queue orders by it, and the dead-person drop reads it.
    pub person: PersonId,
    /// The payload, in the owner's `[[applies]] payload` order.
    pub args: Vec<Arg>,
    /// The event that caused it: a draft of the submitting step, an id read from state, or none.
    /// The merge resolves a draft to its `EventId` ([`QueuedCommand::cause`]), so the owner can
    /// link its own events to it (the death path of `05.1` §2 rule 2).
    pub cause: EventRef,
}

impl Command {
    /// The integer of field `i`, if it is one.
    #[must_use]
    pub fn int(&self, i: usize) -> Option<i64> {
        self.args.get(i).and_then(|a| a.int())
    }

    /// The event id of field `i`, if it is a resolved event slot.
    #[must_use]
    pub fn event(&self, i: usize) -> Option<EventId> {
        self.args
            .get(i)
            .and_then(|a| a.event())
            .and_then(EventRef::known)
    }
}

/// The queue order key: `(tick, phase, person, seq)`.
pub type CommandKey = (SimTime, Phase, PersonId, u64);

/// A merged command, as its owner receives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedCommand {
    /// The instant it was submitted at.
    pub tick: SimTime,
    /// The phase it was submitted in.
    pub phase: Phase,
    /// The merge counter.
    pub seq: u64,
    /// The submitting system.
    pub sender: &'static str,
    /// The `EventId` of the command's cause, resolved from [`Command::cause`].
    pub cause: Option<EventId>,
    /// The draft index the submitting step named its cause by, if it named one by a draft: kept
    /// for the hashed row's `opt(u64 draft)` field (`specs/chronicle/envelope_v0.md` §2), whose
    /// layout SPR-014 does not change.
    pub cause_draft: Option<DraftRef>,
    /// The command, every event slot resolved (`Known` or `None`).
    pub command: Command,
}

impl QueuedCommand {
    /// Its queue order key.
    #[must_use]
    pub fn key(&self) -> CommandKey {
        (self.tick, self.phase, self.command.person, self.seq)
    }
}

/// The serialisable state of the queues (SPR-007 T-02): each owner with its commands, owners in
/// increasing name order and each queue in key order, and the merge counter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandQueuesState {
    /// `(owner, commands)`, owners strictly increasing, every queue non-empty and in key order.
    pub queues: Vec<(&'static str, Vec<QueuedCommand>)>,
    /// The next merge `seq`.
    pub next_seq: u64,
}

/// The queues of every owner: owner id to its commands in key order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandQueues {
    queues: BTreeMap<&'static str, BTreeMap<CommandKey, QueuedCommand>>,
    next_seq: u64,
}

impl CommandQueues {
    /// Empty queues.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues `command` from `sender`, submitted at `(tick, phase)`, for `owner`.
    pub(crate) fn push(
        &mut self,
        owner: &'static str,
        tick: SimTime,
        phase: Phase,
        sender: &'static str,
        cause_draft: Option<DraftRef>,
        command: Command,
    ) {
        let queued = QueuedCommand {
            tick,
            phase,
            seq: self.next_seq,
            sender,
            cause: command.cause.known(),
            cause_draft,
            command,
        };
        self.next_seq += 1;
        self.queues
            .entry(owner)
            .or_default()
            .insert(queued.key(), queued);
    }

    /// Takes `owner`'s whole queue, in key order.
    #[cfg(test)]
    pub(crate) fn take(&mut self, owner: &'static str) -> Vec<QueuedCommand> {
        self.queues
            .remove(owner)
            .map(|q| q.into_values().collect())
            .unwrap_or_default()
    }

    /// Takes the commands of `owner`'s queue whose kind `takes`, in key order; the others stay
    /// queued, in order (`05.1` §2 rule 2: a binding drains only the kinds it is `drained_by`).
    pub(crate) fn take_kinds(
        &mut self,
        owner: &'static str,
        takes: impl Fn(&str) -> bool,
    ) -> Vec<QueuedCommand> {
        let Some(q) = self.queues.get_mut(owner) else {
            return Vec::new();
        };
        if q.values().all(|c| !takes(c.command.kind)) {
            return Vec::new();
        }
        let (drained, kept): (BTreeMap<_, _>, BTreeMap<_, _>) = std::mem::take(q)
            .into_iter()
            .partition(|(_, c)| takes(c.command.kind));
        *q = kept;
        if q.is_empty() {
            self.queues.remove(owner);
        }
        drained.into_values().collect()
    }

    /// Every queued command, owners in name order, each queue in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&'static str, &QueuedCommand)> {
        self.queues
            .iter()
            .flat_map(|(owner, q)| q.values().map(move |c| (*owner, c)))
    }

    /// Puts back commands an owner did not take (they keep their keys and order).
    pub(crate) fn restore(&mut self, owner: &'static str, commands: Vec<QueuedCommand>) {
        if commands.is_empty() {
            return;
        }
        let q = self.queues.entry(owner).or_default();
        for c in commands {
            q.insert(c.key(), c);
        }
    }

    /// The serialisable state, in canonical form.
    #[must_use]
    pub fn state(&self) -> CommandQueuesState {
        CommandQueuesState {
            queues: self
                .queues
                .iter()
                .filter(|(_, q)| !q.is_empty())
                .map(|(owner, q)| (*owner, q.values().cloned().collect()))
                .collect(),
            next_seq: self.next_seq,
        }
    }

    /// Rebuilds the queues from a state in the canonical form [`CommandQueues::state`] produces
    /// (SPR-006 GAP-6, ruled for SPR-007 T-02).
    ///
    /// # Errors
    /// A message unless the owners are strictly increasing, every queue is non-empty and strictly
    /// increasing in key order, and every `seq` is below the counter and appears once.
    pub fn from_state(state: &CommandQueuesState) -> Result<Self, String> {
        if let Some(w) = state.queues.windows(2).find(|w| w[0].0 >= w[1].0) {
            return Err(format!(
                "command owners are not strictly increasing: {} then {}",
                w[0].0, w[1].0
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut queues = BTreeMap::new();
        for (owner, commands) in &state.queues {
            if commands.is_empty() {
                return Err(format!("the queue of {owner} is empty"));
            }
            if let Some(w) = commands.windows(2).find(|w| w[0].key() >= w[1].key()) {
                return Err(format!(
                    "the queue of {owner} is not in strictly increasing key order at seq {} then {}",
                    w[0].seq, w[1].seq
                ));
            }
            for c in commands {
                if c.seq >= state.next_seq || !seen.insert(c.seq) {
                    return Err(format!(
                        "command seq {} is repeated or not below the counter {}",
                        c.seq, state.next_seq
                    ));
                }
            }
            queues.insert(
                *owner,
                commands.iter().map(|c| (c.key(), c.clone())).collect(),
            );
        }
        Ok(Self {
            queues,
            next_seq: state.next_seq,
        })
    }

    /// The number of commands waiting for `owner`.
    #[must_use]
    pub fn waiting(&self, owner: &str) -> usize {
        self.queues.get(owner).map_or(0, BTreeMap::len)
    }

    /// The number of commands waiting in all queues.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queues.values().map(BTreeMap::len).sum()
    }

    /// Whether every queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, CommandQueues};
    use crate::phase::Phase;
    use crate::time::SimTime;
    use rh_ids::PersonId;

    fn cmd(person: u64) -> Command {
        Command {
            kind: "Die",
            person: PersonId::from_raw(person),
            args: vec![],
            cause: rh_events::EventRef::None,
        }
    }

    #[test]
    fn an_owner_gets_its_queue_in_tick_phase_person_seq_order() {
        let mut q = CommandQueues::new();
        let t = SimTime::from_minutes;
        q.push("M2", t(1), Phase::Agenda, "T1", None, cmd(5));
        q.push("M2", t(1), Phase::Vital, "M5", None, cmd(9));
        q.push("M2", t(1), Phase::Vital, "M5", None, cmd(3));
        q.push("M2", t(0), Phase::Military, "M8", None, cmd(7));
        q.push("M2", t(1), Phase::Vital, "M8", None, cmd(3));
        q.push("S1", t(0), Phase::Activity, "S5", None, cmd(1));
        assert_eq!(q.waiting("M2"), 5);
        let got: Vec<(i64, Phase, u64, &str)> = q
            .take("M2")
            .iter()
            .map(|c| {
                (
                    c.tick.minutes(),
                    c.phase,
                    c.command.person.to_raw(),
                    c.sender,
                )
            })
            .collect();
        assert_eq!(
            got,
            [
                (0, Phase::Military, 7, "M8"),
                (1, Phase::Vital, 3, "M5"),
                (1, Phase::Vital, 3, "M8"),
                (1, Phase::Vital, 9, "M5"),
                (1, Phase::Agenda, 5, "T1"),
            ]
        );
        assert_eq!(q.waiting("M2"), 0);
        assert_eq!(q.len(), 1, "S1's queue is untouched");
    }

    /// SPR-007 T-02 (SPR-006 GAP-6): the state round-trips, and only the canonical form restores.
    #[test]
    fn state_round_trips_and_only_the_canonical_form_restores() {
        use super::CommandQueuesState;
        let mut q = CommandQueues::new();
        let t = SimTime::from_minutes;
        q.push("M2", t(1), Phase::Vital, "M5", None, cmd(9));
        q.push("M2", t(0), Phase::Vital, "M5", None, cmd(3));
        q.push("S1", t(0), Phase::Activity, "S5", None, cmd(1));
        q.push("A0", t(2), Phase::Agenda, "S5", None, cmd(4));
        let state = q.state();
        assert_eq!(CommandQueues::from_state(&state), Ok(q.clone()));
        assert_eq!(
            state.queues.iter().map(|(o, _)| *o).collect::<Vec<_>>(),
            ["A0", "M2", "S1"]
        );
        let refused = |f: fn(&mut CommandQueuesState)| {
            let mut s = state.clone();
            f(&mut s);
            CommandQueues::from_state(&s).is_err()
        };
        assert!(refused(|s| s.queues.swap(0, 1)), "unsorted owners");
        assert!(refused(|s| s.queues[1].1.reverse()), "an unsorted queue");
        assert!(refused(|s| s.queues[0].1.clear()), "an empty queue");
        assert!(refused(|s| s.next_seq = 2), "a seq not below the counter");
        assert!(
            refused(|s| {
                let dup = s.queues[1].1[0].clone();
                s.queues[2].1.push(dup);
            }),
            "a repeated seq"
        );
    }
}
