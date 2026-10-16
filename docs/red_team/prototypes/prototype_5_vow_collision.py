"""Prototype 5: The Vow Collision & Moral Dilemma Deadlock

Demonstrates what happens when an agent's sworn vows violently collide.
Compares naive utility AI (infinite oscillation deadlock) against
deterministic hierarchical vow arbitration and tragic renunciation.
"""

from dataclasses import dataclass
from enum import Enum
from typing import List, Optional

class VowTier(Enum):
    BLOOD_OATH = 1       # Sacred kinship / vengeance
    FEALTY_OATH = 2      # Royal allegiance
    CHIVALRIC_DUTY = 3   # Knighthood code
    CONTRACT = 4         # Guild / money debt

@dataclass
class SwornVow:
    id: int
    tier: VowTier
    target_entity_id: int
    verb: str            # e.g., "DEFEND", "SLAY"
    description: str

class KnightAgent:
    def __init__(self, name="Sir Gareth"):
        self.name = name
        self.stress = 10
        self.honor = 90
        self.vows: List[SwornVow] = [
            SwornVow(1, VowTier.FEALTY_OATH, target_entity_id=10, verb="DEFEND", description="Defend King Alden with my life"),
            SwornVow(2, VowTier.BLOOD_OATH, target_entity_id=10, verb="SLAY", description="Slay the killer of my father (King Alden)")
        ]
        self.status = "Active Knight"
        self.chronicle = []

    def log(self, msg: str):
        self.chronicle.append(msg)

    def simulate_naive_ai(self, ticks=4):
        self.log("--- RUN A: NAIVE UTILITY AI (NO ARBITRATION) ---")
        for tick in range(1, ticks + 1):
            # Utility function alternates based on closest evaluation
            if tick % 2 == 1:
                action = "DRAW_SWORD_AGAINST_KING"
                reason = "Evaluating Blood Oath (Slay Father's Killer)"
            else:
                action = "SHIELD_KING_FROM_BLOWS"
                reason = "Evaluating Fealty Oath (Defend King to Death)"
            self.log(f"[Tick {tick}] Action: {action:28s} | Motive: {reason}")
        self.log(">>> FAILURE MODE: Infinite behavior oscillation / cognitive thrashing! <<<\n")

    def simulate_hierarchical_arbitration(self):
        self.log("--- RUN B: RED TEAM HIERARCHICAL VOW ARBITRATION ---")
        self.log(f"Evaluating Vow Conflict for {self.name}:")
        self.log(f"  * Vow 1: [{self.vows[0].tier.name}] - {self.vows[0].description}")
        self.log(f"  * Vow 2: [{self.vows[1].tier.name}] - {self.vows[1].description}")
        
        # Priority check: BLOOD_OATH (Tier 1) > FEALTY_OATH (Tier 2)
        v1, v2 = self.vows[0], self.vows[1]
        dominant_vow = v1 if v1.tier.value < v2.tier.value else v2
        subordinate_vow = v2 if dominant_vow == v1 else v1

        self.log(f"  >> ARBITRATION RULE: {dominant_vow.tier.name} strictly outranks {subordinate_vow.tier.name}!")
        self.log(f"  >> Action Chosen: {dominant_vow.verb} based on {dominant_vow.description}")

        # The Tragic Renunciation & Consequence
        self.log("\n[EXECUTION OF TRAGIC RENUNCIATION]")
        self.log(f"  1. {self.name} formally breaks his [{subordinate_vow.tier.name}].")
        self.honor -= 40
        self.stress += 60
        self.log(f"  2. Honor penalized: -40 -> Honor = {self.honor}")
        self.log(f"  3. Stress surged: +60 -> Stress = {self.stress}/100")
        self.log(f"  4. Reputational Flag assigned: 'Oathbreaker / Kingslayer'")
        self.log(f"  5. {self.name} steps forward, breaks his knightly spurs, and draws his dagger.")
        self.status = "Traitor Knight (Blood Vindicated)"

def run_vow_collision_prototype():
    print("================================================================================")
    print("PROTOTYPE 5: VOW COLLISION & MORAL DILEMMA BENCHMARK")
    print("================================================================================\n")
    
    agent = KnightAgent()
    agent.simulate_naive_ai(ticks=4)
    agent.simulate_hierarchical_arbitration()
    
    for line in agent.chronicle:
        print(line)

    print("\n--- RED TEAM GAP ANALYSIS FOR S7.vows ---")
    print("1. Strict Priority Hierarchy: Vows must have explicit tiers (Blood > Fealty > Chivalry > Contract).")
    print("2. Renunciation Protocol: Breaking a subordinate vow must have explicit deterministic costs (Honor loss, Oathbreaker trait).")
    print("3. Causal Narrative Richness: Rather than an AI deadlock, the collision produces an instant classic mythic tragedy (The Broken Spur).")

if __name__ == "__main__":
    run_vow_collision_prototype()
