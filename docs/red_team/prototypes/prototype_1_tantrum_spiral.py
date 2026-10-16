"""Prototype 1: The Court Tantrum & Grief Cascade

Demonstrates what happens when high-stress events occur in a court network.
Evaluates whether CK3-style stress accumulation without social dampening
causes an unrealistic extinction-level bloodbath (the Dwarf Fortress Tantrum Spiral),
and proves the necessity of Institutional Circuit Breakers.
"""

from dataclasses import dataclass, field
from typing import List, Dict, Optional

@dataclass
class Noble:
    id: int
    name: str
    title: str
    stress: int = 0                  # 0 to 100
    vow: Optional[str] = None        # e.g., "PROTECT_KING", "AVENGE_BROTHER"
    loyalty_to_crown: int = 50       # 0 to 100
    kin_ids: List[int] = field(default_factory=list)
    alive: bool = True
    mental_break_count: int = 0
    status: str = "Calm"

class CourtSimulation:
    def __init__(self):
        self.nobles: Dict[int, Noble] = {
            10: Noble(10, "King Alden", "The Iron Monarch", loyalty_to_crown=100, kin_ids=[42, 15]),
            42: Noble(42, "Prince Eamon", "The Vowed Heir", stress=70, vow="AVENGE_BROTHER", loyalty_to_crown=10, kin_ids=[10, 15]),
            15: Noble(15, "Princess Lyra", "Daughter of Alden", stress=20, loyalty_to_crown=90, kin_ids=[10, 42]),
            20: Noble(20, "Duke Valerius", "Brother of the King", stress=30, loyalty_to_crown=80, kin_ids=[10]),
            31: Noble(31, "Lord Commander Garrett", "Marshal", stress=15, vow="PROTECT_KING", loyalty_to_crown=95),
            55: Noble(55, "Chancellor Malakor", "High Priest", stress=10, loyalty_to_crown=75),
            62: Noble(62, "Lady Seraphina", "Spymaster", stress=40, loyalty_to_crown=40),
            77: Noble(77, "Baron Robert", "Commander of the Gate", stress=25, loyalty_to_crown=60),
        }
        self.log: List[str] = []

    def record(self, msg: str):
        self.log.append(msg)

    def trigger_regicide(self):
        """Prince Eamon poisons King Alden."""
        self.record("\n=== EVENT: REGICIDE IN THE GREAT HALL (Tick 0) ===")
        self.record("Prince Eamon slips Nightshade into King Alden's cup. The King collapses and dies!")
        self.nobles[10].alive = False
        self.nobles[10].status = "Dead (Poisoned)"
        self.nobles[42].stress = 95
        self.nobles[42].status = "Guilty & Vindicated"

    def run_cascade_without_dampening(self, ticks=3):
        """Simulates raw, un-damped stress cascade."""
        self.record("\n--- SIMULATION RUN A: RAW UN-DAMPED STRESS CASCADE ---")
        for tick in range(1, ticks + 1):
            self.record(f"\n[Tick {tick}] Court Dynamics Processing:")
            
            for nid, noble in list(self.nobles.items()):
                if not noble.alive:
                    continue
                
                if not self.nobles[10].alive and noble.id != 42:
                    shock = (noble.loyalty_to_crown // 2)
                    noble.stress = min(100, noble.stress + shock)
                    self.record(f"  * {noble.name} ({noble.title}): Stress spiked +{shock} -> {noble.stress}/100")
                
                if noble.stress >= 100:
                    noble.mental_break_count += 1
                    self.record(f"  !! MENTAL BREAK: {noble.name} suffers breakdown #{noble.mental_break_count}!")
                    
                    if noble.vow == "PROTECT_KING" or noble.loyalty_to_crown > 85:
                        self.record(f"    -> {noble.name} flies into Berserk Fury! Draws sword to execute Prince Eamon!")
                        if self.nobles[42].alive:
                            self.nobles[42].alive = False
                            self.nobles[42].status = "Executed on the spot"
                            self.record(f"    -> Prince Eamon is SLAIN by {noble.name}!")
                    elif noble.id == 15:
                        self.record(f"    -> {noble.name} is paralyzed by Catatonic Despair, burns her personal chapel!")
                    elif noble.id == 20:
                        self.record(f"    -> {noble.name} declares the entire court traitors! Calls household guard to seize the keep!")
                        self.nobles[55].alive = False
                        self.nobles[55].status = "Killed in faction melee"
                        self.record(f"    -> Chancellor Malakor is slaughtered in the scramble!")
                    noble.stress = 50
            
            alive_count = sum(1 for n in self.nobles.values() if n.alive)
            self.record(f"End of Tick {tick}: {alive_count}/8 nobles alive.")
            if alive_count <= 4:
                self.record("  >>> COURT COLLAPSE: 50%+ of leadership dead in under 2 ticks! <<<")
                break

    def run_cascade_with_red_team_circuit_breakers(self, ticks=3):
        """Simulates court with Red Team Circuit Breakers."""
        self.__init__()
        self.nobles[10].alive = False
        self.nobles[10].status = "Dead (Poisoned)"
        self.nobles[42].stress = 80
        self.record("\n--- SIMULATION RUN B: WITH RED TEAM CIRCUIT BREAKERS ---")
        
        for tick in range(1, ticks + 1):
            self.record(f"\n[Tick {tick}] Hardened Dynamics Processing:")
            
            if tick == 1:
                marshal = self.nobles[31]
                self.record(f"  [CIRCUIT BREAKER: MARTIAL AUTHORITY] {marshal.name} draws steel and bars the heavy doors.")
                self.record("  'By the King's blood, no man leaves or draws blade until the truth is sworn!'")
                self.record("  -> Result: Physical violence locked. Stress converts to Suspicion rather than Berserk.")
                for nid, n in self.nobles.items():
                    if n.alive and nid not in [10, 42]:
                        n.stress = min(80, n.stress + 25)
                        self.record(f"  * {n.name}: Stress +25 (Capped at {n.stress}/100, Suspicion active)")
            
            elif tick == 2:
                self.record("  [CIRCUIT BREAKER: DYNASTIC INQUEST] Chancellor Malakor invokes Royal Law.")
                self.record("  Formal interrogation of the Cupbearer and Prince Eamon.")
                spymaster = self.nobles[62]
                self.record(f"  Spymaster {spymaster.name} searches the chambers: Discovers Nightshade phial residue.")
                self.record("  Prince Eamon is arrested and bound in chains (not lynched).")
                self.nobles[42].status = "Imprisoned in Dungeon"
            
            elif tick == 3:
                valerius = self.nobles[20]
                self.record(f"  [SUCCESSION CASSETTE] Duke {valerius.name} claims the Regency by Blood Right.")
                self.record("  Princess Lyra assents to avoid civil war. The realm stabilizes under Regent Valerius.")
                self.record("  Prince Eamon stripped of inheritance, awaiting execution.")
                break
        
        alive_count = sum(1 for n in self.nobles.values() if n.alive)
        self.record(f"\nEnd of Hardened Simulation: {alive_count}/8 nobles alive. Court survived.")

def run_tantrum_prototype():
    sim = CourtSimulation()
    sim.trigger_regicide()
    sim.run_cascade_without_dampening(ticks=3)
    
    sim2 = CourtSimulation()
    sim2.run_cascade_with_red_team_circuit_breakers(ticks=3)
    
    for line in sim.log:
        print(line)
    for line in sim2.log:
        print(line)

if __name__ == "__main__":
    run_tantrum_prototype()
