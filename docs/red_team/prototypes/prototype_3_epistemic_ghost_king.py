"""Prototype 3: The Ghost King & Epistemic Lag Crisis

Demonstrates why agent cognition must be decoupled from Plane A ground truth.
Simulates information transmission delay along a road network, showing how
delayed or false reports trigger preemptive usurpation and tragic civil war.
"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional

@dataclass
class InformationToken:
    subject_id: int
    believed_status: str       # "ALIVE", "DEAD", "WOUNDED"
    origin_tick: int
    source_actor: str
    confidence: float          # 0.0 to 1.0

@dataclass
class AgentBeliefState:
    agent_id: int
    name: str
    location: str
    beliefs: Dict[int, InformationToken] = field(default_factory=dict)
    actions_taken: List[str] = field(default_factory=list)

class RoadNetworkSimulation:
    def __init__(self):
        # Ground truth in Plane A
        self.plane_a_king_alive = True
        self.plane_a_king_location = "NORTHERN_BORDER"
        
        # Distance: Northern Border to Capital = 150 miles
        # Travel speed = 30 miles per tick (1 tick = 1 day) -> Latency = 5 ticks
        self.road_latency_ticks = 5
        
        # Agents
        self.valerius = AgentBeliefState(20, "Duke Valerius", "CAPITAL")
        self.queen = AgentBeliefState(11, "Queen Eleanor", "CAPITAL")
        
        # Information in transit: list of (arrival_tick, token, destination_agent)
        self.transit_queue: List[tuple[int, InformationToken, AgentBeliefState]] = []
        self.chronicle: List[str] = []

    def log(self, tick: int, msg: str):
        self.chronicle.append(f"[Day/Tick {tick:02d}] {msg}")

    def dispatch_report(self, departure_tick: int, token: InformationToken, dest_agent: AgentBeliefState):
        arrival_tick = departure_tick + self.road_latency_ticks
        self.transit_queue.append((arrival_tick, token, dest_agent))

    def step(self, current_tick: int):
        # 1. Process arriving information
        arrived = [item for item in self.transit_queue if item[0] == current_tick]
        self.transit_queue = [item for item in self.transit_queue if item[0] > current_tick]
        
        for _, token, agent in arrived:
            agent.beliefs[token.subject_id] = token
            self.log(current_tick, f"MESSENGER ARRIVES at {agent.location}: Delivers report from {token.source_actor} -> King Alden is {token.believed_status} (Confidence: {token.confidence*100:.0f}%)")

        # 2. Agent Decision Logic based on SUBJECTIVE BELIEFS (not Ground Truth)
        if current_tick == 10:
            # Skirmish at Northern Border: King Alden is knocked unconscious, his retinue flees
            self.plane_a_king_alive = True # HE IS ALIVE!
            self.log(current_tick, "GROUND TRUTH: King Alden wounded at Black Ford, unconscious in hermit's hut. HE IS ALIVE.")
            # Cowardly scout flees south, believing the King fell
            false_report = InformationToken(subject_id=10, believed_status="DEAD", origin_tick=10, source_actor="Fleeing Scout", confidence=0.90)
            self.dispatch_report(10, false_report, self.valerius)
            self.dispatch_report(10, false_report, self.queen)

        # Evaluate Valerius at Capital
        valerius_king_belief = self.valerius.beliefs.get(10)
        if valerius_king_belief and valerius_king_belief.believed_status == "DEAD":
            if "CLAIMED_REGENCY" not in self.valerius.actions_taken:
                self.valerius.actions_taken.append("CLAIMED_REGENCY")
                self.log(current_tick, "POLITICAL CRISIS: Duke Valerius invokes M7 Dynasty. Believing the King dead, he crowns himself Regent to preserve order!")
            elif current_tick == 18 and "PURGED_OPPONENTS" not in self.valerius.actions_taken:
                self.valerius.actions_taken.append("PURGED_OPPONENTS")
                self.log(current_tick, "USURPATION CONSOLIDATION: Valerius arrests Queen's chancellors who accuse him of opportunistic treason.")

        # King awakens and marches home
        if current_tick == 25:
            self.log(current_tick, "GROUND TRUTH: King Alden awakens, rallies 500 loyal veterans, and begins march to Capital.")
            self.plane_a_king_location = "CAPITAL_GATES"
            
        if current_tick == 30:
            self.log(current_tick, "CLIMAX: King Alden arrives at Capital Gates with standard flying.")
            # What does Valerius see?
            valerius_token = self.valerius.beliefs.get(10)
            self.log(current_tick, f"GATE CONFRONTATION: Valerius looks from battlements. His belief is still: King is {valerius_token.believed_status}.")
            self.log(current_tick, "TRAGIC PARADOX: Valerius shouts: 'Impostor! The King fell at Black Ford! You fly a dead man's banner!'")
            self.log(current_tick, "CIVIL WAR ERUPTS: Archers loose volleys upon their own lawful King!")

def run_epistemic_prototype():
    print("================================================================================")
    print("PROTOTYPE 3: THE GHOST KING & EPISTEMIC LAG SIMULATION")
    print("================================================================================\n")
    sim = RoadNetworkSimulation()
    for tick in range(1, 32):
        sim.step(tick)
    
    for line in sim.chronicle:
        print(line)
        
    print("\n--- RED TEAM GAP ANALYSIS FOR AGENT COGNITION ---")
    print("1. Ground Truth vs Belief Separation: Plane A state CANNOT be directly read by agent utility AI.")
    print("2. Epistemic Graph: Every agent requires a lightweight `BeliefStore { entity_id -> (State, Confidence, Timestamp) }`.")
    print("3. Tragic Irony Generation: Sagas like Romeo & Juliet or Oedipus require characters acting reasonably on flawed information.")

if __name__ == "__main__":
    run_epistemic_prototype()
