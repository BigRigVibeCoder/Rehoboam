"""Prototype 2: Story Sifter Signal-to-Noise & Motif Extraction

Demonstrates how rh-sifter extracts epic sagas from thousands of mundane events
using graph-theoretic motif matching on the Arrow/Parquet chronicle stream.
"""

from dataclasses import dataclass, field
from typing import List, Dict, Optional, Set

@dataclass
class CausalEdge:
    source_id: int
    relation: str  # e.g., "AVENGE_KIN", "DESECRATED_BY", "CORRUPTED_BY", "TRADE_DEBT"

@dataclass
class ChronicleEvent:
    event_id: int
    tick: int
    year: int
    actor_id: int
    actor_name: str
    action: str
    target_id: Optional[int] = None
    target_name: Optional[str] = None
    place: str = "TOWN"
    emotion_before: str = "Neutral"
    emotion_after: str = "Neutral"
    edges: List[CausalEdge] = field(default_factory=list)

class EventStreamGenerator:
    """Generates 1,000 mundane events and embeds 3 canonical mythic sagas."""
    def __init__(self):
        self.events: List[ChronicleEvent] = []
        self.current_id = 1

    def generate_stream(self) -> List[ChronicleEvent]:
        # 1. Generate 950 mundane economic & life events
        for tick in range(1, 951):
            year = tick // 50 + 1
            actor = (tick % 30) + 100 # Villagers 100-129
            action = ["HARVEST_WHEAT", "SELL_WOOL", "DRINK_AT_TAVERN", "REPAIR_CART", "PATROL_STREET"][tick % 5]
            ev = ChronicleEvent(
                event_id=self.current_id,
                tick=tick,
                year=year,
                actor_id=actor,
                actor_name=f"Villager_{actor}",
                action=action,
                place="Market_Quarter"
            )
            # Add mundane causal links (buying follows selling)
            if tick > 5 and tick % 7 == 0:
                ev.edges.append(CausalEdge(source_id=self.current_id - 3, relation="TRADE_DEBT"))
            self.events.append(ev)
            self.current_id += 1

        # 2. Embed SAGA 1: The Blood Vendetta (Spans 14 Years / 700 ticks)
        # Event 1: Unjust execution of Kaelan (Tick 50, Year 1)
        ev1 = ChronicleEvent(self.current_id, 50, 1, 10, "King Alden", "EXECUTE_PRISONER", 40, "Kaelan the Pure", "Great_Hall", "SternJustice", "Unyielding")
        self.events.append(ev1); id1 = self.current_id; self.current_id += 1
        
        # Event 2: Prince Eamon swears secret vengeance (Tick 52, Year 1)
        ev2 = ChronicleEvent(self.current_id, 52, 1, 42, "Prince Eamon", "SWEAR_VENGEANCE_VOW", 40, "Kaelan the Pure", "Crypt_Sanctuary", "Grief", "ColdHatred", [CausalEdge(id1, "AVENGE_KIN")])
        self.events.append(ev2); id2 = self.current_id; self.current_id += 1
        
        # Event 3: 14 years later, Eamon poisons the King (Tick 750, Year 15)
        ev3 = ChronicleEvent(self.current_id, 750, 15, 42, "Prince Eamon", "ASSASSINATE_POISON", 10, "King Alden", "High_Keep", "SuppressedPanic", "ColdTriumph", [CausalEdge(id2, "FULFILL_VOW")])
        self.events.append(ev3); id3 = self.current_id; self.current_id += 1
        
        # Event 4: King's final realization (Tick 751, Year 15)
        ev4 = ChronicleEvent(self.current_id, 751, 15, 10, "King Alden", "EPISTEMIC_REALIZATION", 42, "Prince Eamon", "High_Keep", "Agony", "BetrayedHorror", [CausalEdge(id3, "DYING_COGNITION")])
        self.events.append(ev4); self.current_id += 1

        # 3. Embed SAGA 2: The Tragic Fall (Hubris -> Sacrilege -> Madness)
        ev5 = ChronicleEvent(self.current_id, 200, 4, 30, "Warlord Bran", "DESECRATE_ALTAR", 99, "Ancient God", "Sacred_Grove", "ArrogantPride", "Defiant", [])
        self.events.append(ev5); id5 = self.current_id; self.current_id += 1
        
        ev6 = ChronicleEvent(self.current_id, 205, 4, 30, "Warlord Bran", "SUFFER_MADNESS_BREAK", None, None, "Sacred_Grove", "Defiant", "DelusionalTerror", [CausalEdge(id5, "SACRILEGE_CURSE")])
        self.events.append(ev6); id6 = self.current_id; self.current_id += 1
        
        ev7 = ChronicleEvent(self.current_id, 210, 4, 30, "Warlord Bran", "SLAY_OWN_KINSMAN", 31, "Bran Brother", "Warlord_Tent", "DelusionalTerror", "BrokenSorrow", [CausalEdge(id6, "MADNESS_IMPULSE")])
        self.events.append(ev7); self.current_id += 1

        # Sort all events chronologically by tick
        self.events.sort(key=lambda e: e.tick)
        return self.events

class StorySifter:
    """Graph motif matcher identifying dramatic sagas from raw causal graphs."""
    def __init__(self, events: List[ChronicleEvent]):
        self.events = events
        self.event_map = {e.event_id: e for e in events}
        # Build reverse index: child_id -> parent_edges
        self.parent_graph: Dict[int, List[CausalEdge]] = {e.event_id: e.edges for e in events}

    def sifter_naive_search(self) -> List[ChronicleEvent]:
        """Naive filter: looks for any death or violence. Flooded with false positives."""
        return [e for e in self.events if "EXECUTE" in e.action or "ASSASSINATE" in e.action or "SLAY" in e.action]

    def sifter_motif_blood_vendetta(self) -> List[List[ChronicleEvent]]:
        """Motif: Execution -> Vow -> Multi-year Latency (>100 ticks) -> Assassination -> Realization."""
        matched_sagas = []
        for e in self.events:
            if e.action == "EPISTEMIC_REALIZATION":
                # Trace backwards
                chain = [e]
                curr = e
                found = False
                while curr.edges:
                    parent_edge = curr.edges[0]
                    parent_event = self.event_map.get(parent_edge.source_id)
                    if not parent_event:
                        break
                    chain.append(parent_event)
                    curr = parent_event
                    if curr.action == "EXECUTE_PRISONER":
                        found = True
                        break
                if found and len(chain) >= 4:
                    # Check temporal latency: Execution to Regicide > 100 ticks
                    execution_tick = chain[-1].tick
                    assassination_tick = chain[1].tick
                    if assassination_tick - execution_tick >= 100:
                        matched_sagas.append(list(reversed(chain)))
        return matched_sagas

    def sifter_motif_tragic_fall(self) -> List[List[ChronicleEvent]]:
        """Motif: Sacrilege/Desecration -> Madness Break -> Kinslaying."""
        matched_sagas = []
        for e in self.events:
            if e.action == "SLAY_OWN_KINSMAN":
                chain = [e]
                curr = e
                while curr.edges:
                    parent_edge = curr.edges[0]
                    parent_event = self.event_map.get(parent_edge.source_id)
                    if not parent_event:
                        break
                    chain.append(parent_event)
                    curr = parent_event
                    if curr.action == "DESECRATE_ALTAR":
                        matched_sagas.append(list(reversed(chain)))
                        break
        return matched_sagas

def run_sifter_prototype():
    print("================================================================================")
    print("PROTOTYPE 2: STORY SIFTER SIGNAL-TO-NOISE & CAUSAL MOTIF BENCHMARK")
    print("================================================================================\n")
    
    gen = EventStreamGenerator()
    events = gen.generate_stream()
    print(f"1. Total Chronicle Events Generated: {len(events)}")
    print(f"   Mundane peasant economic/patrol events: 950")
    print(f"   Embedded mythic events: 7\n")

    sifter = StorySifter(events)

    # A. The Naive Search Failure
    naive_matches = sifter.sifter_naive_search()
    print(f"2. Naive Filter ('Death / Violence Keyword'): Found {len(naive_matches)} events.")
    print("   Problem: Returns disconnected violence with zero context or narrative arc.\n")

    # B. The Causal Motif Search
    print("3. Executing Causal Graph Motif Queries (rh-sifter pattern matching):")
    
    vendettas = sifter.sifter_motif_blood_vendetta()
    print(f"   [Query: THE BLOOD VENDETTA] -> Matched {len(vendettas)} Saga(s):")
    for idx, saga in enumerate(vendettas, 1):
        print(f"   --- Saga #{idx}: The 14-Year Vendetta of Prince Eamon ---")
        for step in saga:
            print(f"       Tick {step.tick:4d} (Yr {step.year:2d}): {step.actor_name} -> [{step.action}] on {step.target_name} ({step.emotion_before} -> {step.emotion_after})")
    print()

    falls = sifter.sifter_motif_tragic_fall()
    print(f"   [Query: THE TRAGIC FALL] -> Matched {len(falls)} Saga(s):")
    for idx, saga in enumerate(falls, 1):
        print(f"   --- Saga #{idx}: The Sacrilege of Warlord Bran ---")
        for step in saga:
            print(f"       Tick {step.tick:4d} (Yr {step.year:2d}): {step.actor_name} -> [{step.action}] on {step.target_name} ({step.emotion_before} -> {step.emotion_after})")
    print()

    print("--- RED TEAM GAP ANALYSIS FOR rh-sifter ---")
    print("1. Motif Definition Language: We need an explicit TOML/YAML syntax for causal motifs.")
    print("2. Temporal Dilation: Long-latency edges (e.g. 14 years between vow and strike) are critical to distinguishing myth from tavern brawls.")
    print("3. Emotional Inversion Filter: True sagas exhibit radical polarity shifts (Pride -> Terror, Grief -> Vengeance).")

if __name__ == "__main__":
    run_sifter_prototype()
