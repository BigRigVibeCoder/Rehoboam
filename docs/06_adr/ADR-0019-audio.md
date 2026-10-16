# ADR-0019 — Godot-native systemic audio; licence-vetted TTS

**Status:** Proposed · **Date:** 2026-10-15

**Amended by ADR-0035 (2026-10-16):** Godot-native ambience is cut (the 2D diorama mixes in the browser). The audio chronicle is core, not optional: Kokoro-82M narration in P2, multi-voice drama (Chatterbox or Qwen3-TTS) in P3, written by the Python pipeline in plane C (`05.11`). Decision 3's private RSS feed is dropped: an offline exporter writes local MP3 chapters, played in the local saga reader.

## Context

- Runtime neural audio is too slow and competes for the GPU.
- Godot's audio can prevent system sleep.
- Many "open" TTS models are licensed for non-commercial use only.
- espeak-ng is GPL.

## Decision

1. **Ambience**: telemetry-driven layered beds, density beds, and capped one-shots built on Godot buses.
2. **NPC speech**: Chatterbox-Nano/Turbo or Pocket TTS running on CPU, with a bank of designed voices.
3. **Audio drama**:
   - Rendering per line with IndexTTS-2.5, Qwen3-TTS or MOSS-TTSD.
   - An ASR word-error-rate gate on each line.
   - Mastered to −16 LUFS.
   - Delivered as a private RSS feed.
4. **Prohibited**: voice cloning of real people, and in-process espeak-ng.

## Alternatives considered

| Option | Why not |
|---|---|
| FMOD or Wwise | Proprietary, with plugins that lag Godot releases. Optional later. |
| VibeVoice | Code withdrawn. |
| F5, XTTS, Fish S2 | Non-commercial licences. |

## Consequences

- Licence spot-checks are needed for each adopted model.
- The drama ambience comes from the world's own telemetry.

## Revisit when

- An open, commercially licensed multi-speaker model clearly surpasses the current choices.

## Evidence

R07 (all).
