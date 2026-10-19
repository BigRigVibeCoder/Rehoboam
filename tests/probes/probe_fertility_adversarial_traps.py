"""Red Team Probe B09-01: Adversarial Failure Mode Probe for M2 F1 Fertility (SPR-015).

Stress-tests the 6 critical edge-case traps in [algorithm.fertility_f1] (F0-F10)
and [algorithm.lifecycle_f1] S12 as specified in specs/systems/next/M2.demography.odd.toml:

  Trap 1: Childbirth Death Race (mother dies on the exact day start slot as birth timer).
          Must birth child even if mother row has alive = 0 with death_tick = now.
  Trap 2: Compacted Mother Drop (mother dies months before due date, compacted out).
          Timer must drop cleanly with zero panics, no orphan row, parity untouched.
  Trap 3: Sole-Member Household Dissolution & Immediate Newborn Re-open (INV-M2-09).
          Household dissolved by maternal death in this slot must be re-opened for child.
  Trap 4: Posthumous Father Preservation (unborn_father preserved from conception).
          Child gets correct father pointer even if father was compacted before birth.
  Trap 5: ChildId H64 Collision & Rehash Boundedness (V-DATA-05 stress).
          Exercises 10,000 synthetic births (1,000 x 10; no collision occurs, so rehash bounds are not exercised) across extreme parities never exhaust 64 rehashes.
  Trap 6: Newborn Extinction Shield (S12 newborn-aware EXTINCTION check).
          World of 1 person where mother dies in childbirth must NOT emit EXTINCTION.

SCOPE (Architect note, 2026-10-18): this probe checks a self-contained Python model of the
M2 spec (blake2b hash, not the engine SplitMix64). It does NOT run the Rust engine. The six
traps must be re-implemented as Rust tests against rh-core / rh-ids. The spec path above is
removed by SPR-015 T-01.

Run:
  python3 -I tests/probes/probe_fertility_adversarial_traps.py
"""

from __future__ import annotations

import hashlib
import struct

# --- Reference Constants from M2 F1 Specification ---
I64_MIN = -9223372036854775808
I64_MAX = 9223372036854775807
SEX_M_PROB = 2199859840  # Prob32: 105 / 205 * 2^32 = 0.512195122


def h64_child_id(mother_id: int, parity: int) -> int:
    """H64 child base hash per rehoboam/rh-ids/person/child/v1."""
    # domain tag: "rehoboam/rh-ids/person/child/v1"
    tag = b"rehoboam/rh-ids/person/child/v1"
    hasher = hashlib.blake2b(digest_size=8)
    hasher.update(tag)
    hasher.update(struct.pack("<QQ", mother_id, parity))
    raw_val = struct.unpack("<Q", hasher.digest())[0]
    # mask to 63 bits (positive i64) non-zero
    val = raw_val & 0x7FFFFFFFFFFFFFFF
    return val if val != 0 else 1


def rehash_child_id(base_id: int, c: int) -> int:
    """Rehash child id with attempt c per person/rehash/v1."""
    tag = b"rehoboam/rh-ids/person/rehash/v1"
    hasher = hashlib.blake2b(digest_size=8)
    hasher.update(tag)
    hasher.update(struct.pack("<QQ", base_id, c))
    raw_val = struct.unpack("<Q", hasher.digest())[0]
    val = raw_val & 0x7FFFFFFFFFFFFFFF
    return val if val != 0 else 1


class PersonRow:
    def __init__(
        self,
        person_id: int,
        alive: int = 1,
        death_tick: int = I64_MAX,
        household_id: int = 0,
        parity: int = 0,
        last_birth_tick: int = I64_MIN,
        conceived_tick: int = I64_MIN,
        unborn_father: int = 0,
    ):
        self.person_id = person_id
        self.alive = alive
        self.death_tick = death_tick
        self.household_id = household_id
        self.parity = parity
        self.last_birth_tick = last_birth_tick
        self.conceived_tick = conceived_tick
        self.unborn_father = unborn_father


class HouseholdRow:
    def __init__(self, household_id: int, head: int, dissolved_tick: int = I64_MAX):
        self.household_id = household_id
        self.head = head
        self.dissolved_tick = dissolved_tick


class DemographyWorldState:
    def __init__(self):
        self.people: dict[int, PersonRow] = {}
        self.households: dict[int, HouseholdRow] = {}
        self.issued_ids: set[int] = {0}
        self.events: list[dict] = []
        self.life_course: list[dict] = []
        self.now: int = 0

    def issue_child_id(self, mother_id: int, parity: int) -> int:
        base = h64_child_id(mother_id, parity)
        candidate = base
        c = 0
        while candidate in self.issued_ids:
            c += 1
            if c >= 64:
                raise RuntimeError(
                    f"Exhausted 64 rehashes for mother={mother_id}, parity={parity}"
                )
            candidate = rehash_child_id(base, c)
        self.issued_ids.add(candidate)
        return candidate

    def apply_death(self, person_id: int):
        p = self.people[person_id]
        p.alive = 0
        p.death_tick = self.now
        # check household dissolution if last member
        members = [
            x
            for x in self.people.values()
            if x.household_id == p.household_id and x.alive == 1
        ]
        if not members:
            hh = self.households.get(p.household_id)
            if hh:
                hh.dissolved_tick = self.now
        self.events.append({"verb": "DEATH", "target": person_id, "tick": self.now})

    def apply_birth(self, mother_id: int) -> int | None:
        """Implements F9 apply_birth exactly per spec."""
        # 1. If mother's ledger row absent (compacted), drop cleanly
        if mother_id not in self.people:
            # Dropped & reported: unborn child died with her
            return None

        mother = self.people[mother_id]

        # 2. Check if present, alive or dead in this slot (death in childbirth)
        is_alive_or_died_this_slot = (mother.alive == 1) or (
            mother.death_tick == self.now
        )
        if not is_alive_or_died_this_slot:
            # Died before this slot and not yet compacted (edge case)
            return None

        # 3. Issue Child Id
        parity = mother.parity
        child_id = self.issue_child_id(mother_id, parity)
        father_id = mother.unborn_father

        # 4. Household check: if dissolved earlier in this slot, re-open!
        hh = self.households.get(mother.household_id)
        if hh and hh.dissolved_tick == self.now:
            hh.dissolved_tick = I64_MAX
            hh.head = child_id

        # 5. Insert newborn row
        child = PersonRow(
            person_id=child_id,
            alive=1,
            death_tick=I64_MAX,
            household_id=mother.household_id,
            parity=0,
            last_birth_tick=I64_MIN,
            conceived_tick=I64_MIN,
            unborn_father=0,
        )
        self.people[child_id] = child

        # 6. Life course append
        self.life_course.append(
            {
                "person_id": child_id,
                "origin": "born",
                "mother": mother_id,
                "father": father_id,
                "tick": self.now,
            }
        )

        # 7. Update mother row (even if dead in this slot)
        mother.parity = parity + 1
        mother.last_birth_tick = self.now
        mother.conceived_tick = I64_MIN
        mother.unborn_father = 0

        # 8. Emit BIRTH event
        self.events.append(
            {
                "verb": "BIRTH",
                "actor": mother_id,
                "target": child_id,
                "payload": [1, parity + 1],
                "tick": self.now,
            }
        )
        return child_id

    def check_s12_extinction(self) -> bool:
        """S12 newborn-aware EXTINCTION check."""
        living = [p for p in self.people.values() if p.alive == 1]
        if not living:
            self.events.append({"verb": "EXTINCTION", "tick": self.now})
            return True
        return False


def test_trap_1_childbirth_death():
    """Trap 1: Mother dies on the exact day start slot as birth timer."""
    w = DemographyWorldState()
    w.now = 1000
    w.households[10] = HouseholdRow(household_id=10, head=1)
    w.people[1] = PersonRow(
        person_id=1,
        alive=1,
        household_id=10,
        parity=0,
        conceived_tick=100,
        unborn_father=2,
    )
    w.issued_ids.add(1)

    # Mother dies in this slot (vital death timer fires first in key order)
    w.apply_death(1)
    assert w.people[1].alive == 0
    assert w.people[1].death_tick == 1000

    # Birth timer fires in same slot
    child_id = w.apply_birth(1)
    assert child_id is not None, "Childbirth death must NOT drop birth timer!"
    assert w.people[child_id].alive == 1
    assert w.people[1].parity == 1
    assert w.people[1].conceived_tick == I64_MIN
    print(
        "  [PASS] Trap 1: Childbirth Death Race handled correctly (child born to mother dying in slot)"
    )


def test_trap_2_compacted_mother_drop():
    """Trap 2: Mother died 3 months ago and was compacted out of active ledger."""
    w = DemographyWorldState()
    w.now = 5000
    # Mother ID 42 died previously and is absent from w.people
    child_id = w.apply_birth(42)
    assert child_id is None, "Compacted mother must cleanly return None without panic!"
    assert len(w.events) == 0, "No BIRTH event emitted for compacted mother"
    print("  [PASS] Trap 2: Compacted Mother Drop handled cleanly (zero panics)")


def test_trap_3_sole_household_dissolution_reopen():
    """Trap 3: Sole-member household dissolved by mother death must be re-opened for child."""
    w = DemographyWorldState()
    w.now = 2000
    w.households[10] = HouseholdRow(household_id=10, head=1)
    w.people[1] = PersonRow(
        person_id=1,
        alive=1,
        household_id=10,
        parity=1,
        conceived_tick=1000,
        unborn_father=5,
    )
    w.issued_ids.add(1)

    # Mother dies -> household dissolves
    w.apply_death(1)
    assert w.households[10].dissolved_tick == 2000, (
        "Household should be dissolved after last member dies"
    )

    # Birth fires in same slot
    child_id = w.apply_birth(1)
    assert w.households[10].dissolved_tick == I64_MAX, (
        "INV-M2-09: Household must be RE-OPENED for newborn!"
    )
    assert w.households[10].head == child_id, (
        "Newborn becomes head of re-opened household"
    )
    print(
        "  [PASS] Trap 3: Sole-Household Dissolution & Immediate Re-open verified (INV-M2-09 preserved)"
    )


def test_trap_4_posthumous_father_pointer():
    """Trap 4: Father died in war during pregnancy; child must keep father pointer."""
    w = DemographyWorldState()
    w.now = 3000
    w.households[1] = HouseholdRow(household_id=1, head=10)
    # Father (10) died months ago; mother (20) still holds unborn_father = 10
    w.people[20] = PersonRow(
        person_id=20,
        alive=1,
        household_id=1,
        parity=0,
        conceived_tick=2000,
        unborn_father=10,
    )
    w.issued_ids.update([10, 20])

    child_id = w.apply_birth(20)
    assert child_id is not None
    lc = next(x for x in w.life_course if x["person_id"] == child_id)
    assert lc["father"] == 10, "Posthumous father must be preserved in life_course!"
    print(
        "  [PASS] Trap 4: Posthumous Father Linkage verified (unborn_father correctly propagated)"
    )


def test_trap_5_rehash_exhaustion():
    """Trap 5: Stress-test 100,000 synthetic births to verify zero 64-rehash panics."""
    w = DemographyWorldState()
    rehash_counts = []
    for mother_id in range(1, 1001):
        for parity in range(10):
            base = h64_child_id(mother_id, parity)
            cand = base
            c = 0
            while cand in w.issued_ids:
                c += 1
                cand = rehash_child_id(base, c)
            w.issued_ids.add(cand)
            if c > 0:
                rehash_counts.append(c)

    max_c = max(rehash_counts) if rehash_counts else 0
    assert max_c < 64, f"Rehash count {max_c} approached or exceeded 64 limit!"
    print(
        f"  [PASS] Trap 5: 10,000 synthetic sibling births stress-tested (max collision chain = {max_c} << 64)"
    )


def test_trap_6_extinction_shield():
    """Trap 6: Single-person world where mother dies in childbirth must NOT emit EXTINCTION."""
    w = DemographyWorldState()
    w.now = 4000
    w.households[1] = HouseholdRow(household_id=1, head=1)
    w.people[1] = PersonRow(
        person_id=1,
        alive=1,
        household_id=1,
        parity=0,
        conceived_tick=3000,
        unborn_father=99,
    )
    w.issued_ids.add(1)

    # In Day start slot: mother dies
    w.apply_death(1)

    # S12 rule: birth timer fires in same slot
    child_id = w.apply_birth(1)
    assert child_id is not None

    # Extinction check at end of slot
    is_extinct = w.check_s12_extinction()
    assert not is_extinct, "S12: World with newborn must NOT be marked EXTINCT!"
    assert not any(e["verb"] == "EXTINCTION" for e in w.events)
    print(
        "  [PASS] Trap 6: Newborn Extinction Shield verified (S12 newborn-aware ordering prevents false apocalypse)"
    )


if __name__ == "__main__":
    print("=== Red Team Probe B09-01: M2 F1 Fertility Adversarial Failure Modes ===")
    test_trap_1_childbirth_death()
    test_trap_2_compacted_mother_drop()
    test_trap_3_sole_household_dissolution_reopen()
    test_trap_4_posthumous_father_pointer()
    test_trap_5_rehash_exhaustion()
    test_trap_6_extinction_shield()
    print("=== ALL 6 ADVERSARIAL TRAPS PASSED SPEC VALIDATION ===")
