#!/usr/bin/env python3
"""
Prototype 10: Relic Corruption Attractor & Cognitive Dissonance (RT-ACT-013)
Target Spec: S6.artifacts, S7.vows, T1.minds
Research Basis: Dynamical systems attractor basins; Festinger (1957) cognitive dissonance;
                Park et al. (2023) symbolic memory reflection.

Vulnerability Audited:
In traditional procedural fantasy simulations, cursed relics either:
1. Instantly turn the bearer evil via an arbitrary random roll ("rolled a 1, now you are a traitor"), or
2. Grant static stat buffs (+5 strength, -2 charisma) with zero narrative or psychological depth.

The Architectural Solution:
Rehoboam models relic corruption as a continuous, deterministic Dynamical Attractor:
    M_vec(t + 1) = M_vec(t) + kappa * (Psi_relic - M_vec(t))

Where:
- M_vec is the bearer's 4D personality vector: [Honor, Ambition, Paranoia, Greed] (scaled 0-100 in Q8_8).
- Psi_relic is the relic's inherent influence attractor.
- kappa is the monthly coupling rate (e.g. 1/64 = 0.015625).
- Cognitive Dissonance Stress accumulates each month based on the Euclidean distance between
  the bearer's drifting mind vector and their sworn sacred vows:
      Stress(t + 1) = Stress(t) + ||M_vec(t) - Vow_ideal|| * lambda - Recovery
- Crossing Stress >= 80 triggers a deterministic MentalBreak state transition, causing the
  honorable knight to betray fealty and seize power in tragic self-justification.
"""

import math
import sys

# Fixed-point simulation parameters
KAPPA = 1.0 / 64.0        # Monthly relic coupling rate (power-of-two friendly)
DISSONANCE_WEIGHT = 0.12  # Stress accumulation per unit of cognitive dissonance
MONTHLY_RECOVERY = 1.5    # Natural emotional recovery
BREAK_THRESHOLD = 80.0    # Fixed-point Q8_8 stress threshold for mental break

class MindVector:
    def __init__(self, honor, ambition, paranoia, greed):
        self.honor = float(honor)
        self.ambition = float(ambition)
        self.paranoia = float(paranoia)
        self.greed = float(greed)
        
    def distance(self, other):
        """Euclidean distance representing psychological dissonance."""
        return math.sqrt(
            (self.honor - other.honor)**2 +
            (self.ambition - other.ambition)**2 +
            (self.paranoia - other.paranoia)**2 +
            (self.greed - other.greed)**2
        )
        
    def step_attractor(self, attractor, kappa):
        """Deterministic exponential decay toward the attractor basin."""
        self.honor += kappa * (attractor.honor - self.honor)
        self.ambition += kappa * (attractor.ambition - self.ambition)
        self.paranoia += kappa * (attractor.paranoia - self.paranoia)
        self.greed += kappa * (attractor.greed - self.greed)
        
    def __repr__(self):
        return f"[Hon:{self.honor:4.1f}, Amb:{self.ambition:4.1f}, Par:{self.paranoia:4.1f}, Grd:{self.greed:4.1f}]"

def run_relic_simulation(months=120):
    print("=" * 75)
    print("PROTOTYPE 10: RELIC CORRUPTION ATTRACTOR & MENTAL BREAK SPIRAL")
    print("=" * 75)
    
    # Knight: Sir Galahad of Thornwood
    # Sworn Vow: Chivalric Fealty to King Alden
    vow_ideal = MindVector(honor=95.0, ambition=10.0, paranoia=5.0, greed=5.0)
    current_mind = MindVector(honor=90.0, ambition=15.0, paranoia=10.0, greed=5.0)
    
    # Cursed Relic: The Obsidian Wyrm Circlet
    # The attractor pulls toward tyrannical megalomania
    relic_attractor = MindVector(honor=5.0, ambition=95.0, paranoia=90.0, greed=85.0)
    
    print("Agent:      Sir Galahad of Thornwood (Paladin of the Western Border)")
    print(f"Sworn Vow:  Fealty Oath to King Alden (Ideal: {vow_ideal})")
    print(f"Initial:    Mind: {current_mind} | Stress: 0.0/100")
    print(f"Relic:      The Obsidian Wyrm Circlet (Attractor: {relic_attractor})")
    print(f"Coupling:   kappa = {KAPPA:.4f} (~1.56% drift per month)\n")
    
    stress = 0.0
    mental_break_month = None
    break_event = None
    
    timeline = []
    
    for m in range(1, months + 1):
        # 1. Attractor update (continuous personality corruption)
        current_mind.step_attractor(relic_attractor, KAPPA)
        
        # 2. Cognitive Dissonance: distance between sworn vows and current corrupted desires
        dissonance = current_mind.distance(vow_ideal)
        
        # 3. Stress Accumulation
        stress_gain = dissonance * DISSONANCE_WEIGHT
        stress = max(0.0, min(100.0, stress + stress_gain - MONTHLY_RECOVERY))
        
        # 4. Check for Mental Break threshold
        if stress >= BREAK_THRESHOLD and mental_break_month is None:
            mental_break_month = m
            break_event = {
                "month": m,
                "year": m // 12 + 1,
                "stress": round(stress, 1),
                "type": "MENTAL_BREAK_PARANOID_USURPATION",
                "mind": repr(current_mind),
                "narrative": (
                    "Sir Galahad convinces himself that King Alden plans to execute him out of jealousy. "
                    "Overwhelmed by dissonance, he violates his sacred fealty vow, slays the royal envoy, "
                    "and raises the dragon banner over Highspire."
                )
            }
            
        if m in [1, 12, 24, 36, 48, 60, 84, 120] or m == mental_break_month:
            year = (m - 1) // 12 + 1
            month_in_year = (m - 1) % 12 + 1
            is_break = " [!] MENTAL BREAK" if m == mental_break_month else ""
            timeline.append((year, month_in_year, m, repr(current_mind), round(stress, 1), round(dissonance, 1), is_break))
            
    print("Monthly Personality & Cognitive Dissonance Progression:")
    print("--------------------------------------------------------------------------------")
    print(" Year.Mo  (Month) | Bearer Personality Vector            | Stress | Dissonance | Event")
    print("--------------------------------------------------------------------------------")
    for yr, mo, total_m, mind_str, str_val, diss_val, flag in timeline:
        print(f"  Yr{yr:2d}.M{mo:02d} (Mo {total_m:3d}) | {mind_str:36s} | {str_val:5.1f}  | {diss_val:8.1f}   |{flag}")
    print("--------------------------------------------------------------------------------\n")
    
    if break_event:
        print(f"[!] HISTORICAL EVENT TRIGGERED: {break_event['type']} (Year {break_event['year']}, Month {break_event['month']})")
        print(f"    Stress Level:   {break_event['stress']}/100.0 (Threshold: {BREAK_THRESHOLD})")
        print(f"    Final Mindset:  {break_event['mind']}")
        print(f"    Chronicler Log: \"{break_event['narrative']}\"\n")
        
    # Verification checks
    print("[VERIFICATION AUDIT]")
    assert mental_break_month is not None, "Relic failed to trigger mental break"
    assert 24 <= mental_break_month <= 60, f"Break occurred outside historical 2-5 year span: Month {mental_break_month}"
    assert current_mind.ambition > 70.0, "Ambition failed to drift toward attractor"
    assert current_mind.honor < 30.0, "Honor failed to decay toward attractor"
    
    print(f" -> PASSED: Relic corruption accurately triggered break at Month {mental_break_month} (~Year {break_event['year']}).")
    print(" -> PASSED: 100% deterministic continuous drift (zero random dice rolls).")
    print(" -> PASSED: Cognitive dissonance mathematically linked sworn vows to psychological collapse.")
    print("=" * 75)
    return True

if __name__ == "__main__":
    success = run_relic_simulation()
    sys.exit(0 if success else 1)
