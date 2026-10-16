#!/usr/bin/env bash
# Runs all 5 Red Team Fidelity & Edge-Case Prototypes in sequence.
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PYTHON="${PYTHON:-python3}"

echo "================================================================================"
echo "RUNNING REHOBOAM RED TEAM PROTOTYPE SUITE (5/5)"
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

echo -e "\n================================================================================"
echo "ALL 5 PROTOTYPES COMPLETED SUCCESSFULLY."
echo "================================================================================"
