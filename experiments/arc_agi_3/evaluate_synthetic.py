"""Evaluate an ARC-AGI-3 controller on frozen generated environments."""

from __future__ import annotations

import argparse
import json

from synthetic_env import evaluate_split


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--split", choices=("curriculum", "heldout", "all"), default="all")
    args = parser.parse_args()
    splits = ("curriculum", "heldout") if args.split == "all" else (args.split,)
    total_solved = 0
    total_cases = 0
    for split in splits:
        results = evaluate_split(split)
        for result in results:
            print(
                json.dumps(
                    {
                        "split": split,
                        "name": result.name,
                        "solved": result.solved,
                        "actions": result.actions,
                        "resets": result.resets,
                        "interactions": sum(
                            event.get("event") == "interaction" for event in result.events
                        ),
                    },
                    sort_keys=True,
                )
            )
        solved = sum(result.solved for result in results)
        total_solved += solved
        total_cases += len(results)
        print(f"split={split},solved={solved}/{len(results)}")
    print(f"total_solved={total_solved}/{total_cases}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
