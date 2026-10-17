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

use rh_events::DraftRef;
use rh_ids::{EventId, PersonId};

use crate::phase::Phase;
use crate::time::SimTime;

/// A lifecycle command as a system submits it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Command {
    /// The command's Registry name (`[[commands]] command`, e.g. `TieDelta`).
    pub kind: &'static str,
    /// The person it concerns: the queue orders by it.
    pub person: PersonId,
    /// Command-specific integer arguments.
    pub args: Vec<i64>,
    /// The event that caused it, if the submitting step emitted one: resolved to its `EventId` at
    /// the merge ([`QueuedCommand::cause`]), so the owner can link its own events to it (the death
    /// path of `05.1` §2 rule 2).
    pub cause: Option<DraftRef>,
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
    /// The command.
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
        cause: Option<EventId>,
        command: Command,
    ) {
        let queued = QueuedCommand {
            tick,
            phase,
            seq: self.next_seq,
            sender,
            cause,
            command,
        };
        self.next_seq += 1;
        self.queues
            .entry(owner)
            .or_default()
            .insert(queued.key(), queued);
    }

    /// Takes `owner`'s whole queue, in key order.
    pub(crate) fn take(&mut self, owner: &'static str) -> Vec<QueuedCommand> {
        self.queues
            .remove(owner)
            .map(|q| q.into_values().collect())
            .unwrap_or_default()
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
            cause: None,
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
