#!/usr/bin/env bash
# Runs all 9 Red Team Fidelity & Edge-Case Prototypes in sequence.
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON="${PYTHON:-python3}"

echo "================================================================================"
echo "RUNNING REHOBOAM RED TEAM PROTOTYPE SUITE (9/9)"
echo "================================================================================"

echo -e "\n>>> 1. TANTRUM SPIRAL & INSTITUTIONAL CIRCUIT BREAKERS <<<"
$PYTHON "$DIR/prototype_1_tantrum_spiral.py"

echo -e "\n>>> 2. STORY SIFTER CAUSAL MOTIF BENCHMARK <<<"
$PYTHON "$DIR/prototype_2_story_sifter_motifs.py"

echo -e "\n>>> 3. GHOST KING & EPISTEMIC LAG CRISIS <<<"
$PYTHON "$DIR/prototype_3_epistemic_ghost_king.py"

echo -e "\n>>> 4. MICROPOLIS REGIONAL SIEGE & CELLULAR AUTOMATA <<<"
$PYTHON "$DIR/prototype_4_micropolis_siege.py"

echo -e "\n>>> 5. VOW COLLISION & MORAL DILEMMA BENCHMARK <<<"
$PYTHON "$DIR/prototype_5_vow_collision.py"

echo -e "\n>>> 6. BARNES PRIORITY-FLOOD HYDROLOGY & RIVERS <<<"
$PYTHON "$DIR/prototype_6_priority_flood_hydrology.py"

echo -e "\n>>> 7. THE DYNASTIC NOBILITY THERMOSTAT (RT-ACT-010) <<<"
$PYTHON "$DIR/prototype_7_nobility_thermostat.py"

echo -e "\n>>> 8. SQL CAUSAL DAG SIFTER & KREMINSKI UNEXPECTEDNESS (RT-ACT-012) <<<"
$PYTHON "$DIR/prototype_8_causal_sifter_sql.py"

echo -e "\n>>> 9. THE 4-BEAT COMIC STORYBOARD COMPILER (RT-ACT-011) <<<"
$PYTHON "$DIR/prototype_9_comic_beat_compiler.py"

echo -e "\n>>> 10. RELIC CORRUPTION ATTRACTOR & MENTAL BREAK (RT-ACT-013) <<<"
$PYTHON "$DIR/prototype_10_relic_corruption_attractor.py"

echo -e "\n================================================================================"
echo "ALL 10 PROTOTYPES COMPLETED SUCCESSFULLY."
echo "================================================================================"
