"""Run the bounded ontogenesis controller on an official ARC-AGI-3 game."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import numpy as np
from arc_agi import Arcade, OperationMode
from arcengine import GameState

sys.path.insert(0, str(Path(__file__).parent))
from ontogenesis import OntogenesisController


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--game", default="ls20")
    parser.add_argument("--max-actions", type=int, default=80)
    parser.add_argument("--seed", type=int, default=0)
    parser.add_argument("--trace", action="store_true")
    parser.add_argument(
        "--competition",
        action="store_true",
        help="use the official API-only competition protocol",
    )
    args = parser.parse_args()

    mode = OperationMode.COMPETITION if args.competition else OperationMode.NORMAL
    arcade = Arcade(operation_mode=mode)
    env = arcade.make(args.game, seed=args.seed)
    if env is None or env.observation_space is None:
        raise RuntimeError(f"could not create {args.game}")

    controller: OntogenesisController | None = None
    inherited: OntogenesisController | None = None
    level = env.observation_space.levels_completed
    for step in range(args.max_actions):
        observation = env.observation_space
        if observation is None:
            raise RuntimeError("environment returned no observation")
        if observation.state == GameState.WIN:
            break
        if observation.state == GameState.GAME_OVER:
            if controller is not None:
                controller.mark_episode_failure()
            observation = env.reset()
            if controller is not None:
                controller.begin_episode()
            if observation is None:
                break
        if observation.levels_completed != level:
            level = observation.levels_completed
            inherited = controller
            if inherited is not None:
                inherited.mark_level_success()
                print(f"level_transition={level},{inherited.machine_record()}")
            controller = None
        actions = env.action_space
        simple = [action for action in actions if action.is_simple()]
        if not simple:
            raise RuntimeError("this first rung supports simple actions only")
        if controller is None:
            controller = OntogenesisController(
                [action.name for action in simple], inherited=inherited
            )
        name = controller.choose(np.asarray(observation.frame[-1]))
        if args.trace:
            for event in controller.drain_events():
                print(json.dumps(event, sort_keys=True))
        action = next(action for action in simple if action.name == name)
        observation = env.step(action, data={})
        if observation is None:
            raise RuntimeError(f"no observation after {name}")
        print(
            f"step={step + 1},action={name},level={observation.levels_completed},"
            f"state={observation.state.name}"
        )

    assert controller is not None
    print(controller.machine_record())
    if args.competition:
        # Competition mode deliberately forbids reading an in-flight
        # scorecard. Closing the one allowed scorecard is the only scoring
        # operation performed by this process.
        scorecard = arcade.close_scorecard()
    else:
        scorecard = arcade.get_scorecard()
    if scorecard is not None:
        print(
            f"levels_completed={scorecard.total_levels_completed},"
            f"actions={scorecard.total_actions},score={scorecard.score}"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
