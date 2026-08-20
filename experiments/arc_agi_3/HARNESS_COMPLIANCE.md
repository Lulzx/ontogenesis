# ARC-AGI-3 harness compliance

This checklist is an acceptance gate, not a claim that the current solver is
competitive. The controlling sources are the current
[competition rules](https://www.kaggle.com/competitions/arc-prize-2026-arc-agi-3/rules),
[official starter](https://github.com/arcprize/ARC-AGI-3-Kaggle-Starter),
[agent framework](https://github.com/arcprize/ARC-AGI-3-Agents), and
[technical report](https://arcprize.org/media/ARC_AGI_3_Technical_Report.pdf).
If they conflict, the live Kaggle rules and competition gateway control the
submission.

## Runtime gate

- Use the official `Agent` interface and a class named `MyAgent`.
- `is_done(frames, latest_frame)` stops only after `GameState.WIN`.
- `choose_action(frames, latest_frame)` returns exactly one available
  `GameAction` on every turn.
- Return `RESET` for `NOT_PLAYED` and `GAME_OVER`; do not request a full game
  reset. The competition converts reset requests to current-level resets.
- Read observations only from the frame stream, state, levels-completed count,
  and advertised available actions. Do not inspect downloaded environment
  source, engine internals, hidden state, scorecards, recordings, solutions, or
  human baselines while acting.
- Let the gateway enumerate all hidden environments, create the scorecard, and
  write `submission.parquet`. Do not filter games or query an in-flight
  scorecard.
- Keep all runtime dependencies inside the internet-disabled notebook. The
  generated notebook must pass Save & Run All before the competition rerun.
- Bound actions and wall time so every environment terminates and the gateway
  can produce the scorecard.

## Generalization and data gate

- No game-ID branches, fixed action sequences, human replays, hand-labeled
  validation/test records, or configuration selected from private outcomes.
- No private sharing outside the registered Kaggle team. Public competition
  code/data must follow Kaggle's sharing and licensing rules.
- External data, models, and tools must satisfy the competition's availability
  and licensing requirements. A prize-eligible system must be reproducible and
  open-source, including required model weights and documentation.
- A score from public environments is a development diagnostic. Under the
  technical report's methodology it is not evidence of official AGI progress;
  task-specific and domain-specific harness gains are excluded from that
  scientific claim.

## Repository gate

The production path is `ontogenesis.py:MyAgent`. `run.py` is diagnostic only.
Any acceptance report must include:

1. unit-test result;
2. official-starter local harness result across every available environment;
3. generated offline notebook result;
4. competition rerun result, if a submission was deliberately spent; and
5. separate labels for local, public, Kaggle private, and official-model scores.
