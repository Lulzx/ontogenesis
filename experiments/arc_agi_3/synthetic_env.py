"""Deterministic black-box environments for developmental ARC-AGI-3 work.

These environments are not replicas of any ARC-AGI-3 game. They expose only
the same observation/action shape used by :class:`OntogenesisController`: a
64x64 categorical frame and opaque simple actions. Mechanics, layouts, colors,
action bindings, and required compositions are generated from frozen seeds.
"""

from __future__ import annotations

import json
import random
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from ontogenesis import LatentSignature, OntogenesisController

Point = tuple[int, int]
MANIFEST_PATH = Path(__file__).with_name("synthetic_manifests.json")


PATTERNS: tuple[tuple[bool, ...], ...] = (
    (True, True, False, True, False, False, True, True, True),
    (True, False, False, True, True, True, False, False, True),
    (False, True, False, True, True, True, False, True, False),
    (True, True, True, False, True, False, False, True, False),
)


@dataclass(frozen=True)
class SyntheticSpec:
    name: str
    seed: int
    mechanics: tuple[str, ...]
    max_actions: int = 120
    distractors: int = 0


@dataclass
class Entity:
    kind: str
    anchor: Point
    color: int
    destination: Point | None = None
    consumed: bool = False


@dataclass(frozen=True)
class SyntheticResult:
    name: str
    solved: bool
    actions: int
    resets: int
    events: tuple[dict[str, object], ...]


def load_manifest() -> dict[str, tuple[SyntheticSpec, ...]]:
    raw = json.loads(MANIFEST_PATH.read_text())
    return {
        split: tuple(
            SyntheticSpec(
                name=item["name"],
                seed=item["seed"],
                mechanics=tuple(item["mechanics"]),
                max_actions=item.get("max_actions", 120),
                distractors=item.get("distractors", 0),
            )
            for item in items
        )
        for split, items in raw.items()
    }


class SyntheticEnvironment:
    """A generated interactive grid world rendered through public pixels only."""

    stride = 5
    floor = 3
    wall = 8
    panel_background = 5
    budget_color = 11
    action_names = ("ACTION1", "ACTION2", "ACTION3", "ACTION4")

    def __init__(self, spec: SyntheticSpec) -> None:
        self.spec = spec
        rng = random.Random(spec.seed)
        deltas = [(0, -5), (0, 5), (-5, 0), (5, 0)]
        rng.shuffle(deltas)
        self.action_deltas = dict(zip(self.action_names, deltas))
        self.start = (14 + 5 * rng.randrange(2), 5 + 5 * rng.randrange(2))
        self.avatar_color = rng.choice([1, 2, 4, 9, 12])
        self.initial_pattern = PATTERNS[rng.randrange(len(PATTERNS))]
        goal_color = self.avatar_color
        goal_pattern = self.initial_pattern
        if "recolor" in spec.mechanics:
            goal_color = rng.choice(
                [color for color in (1, 2, 4, 9, 12) if color != self.avatar_color]
            )
        turns = 0
        if "rotate" in spec.mechanics:
            turns = rng.choice([1, 2, 3])
            signature = LatentSignature(goal_color, goal_pattern)
            for _ in range(turns):
                signature = signature.rotate_clockwise()
            goal_pattern = signature.pattern
        self.goal_signature = LatentSignature(goal_color, goal_pattern)
        self._rng = rng
        self._entity_colors = iter(rng.sample([6, 7, 10, 13, 14, 15], 6))
        self._build_entities()
        self._build_transition_basin()
        self.total_actions = 0
        self.reset()

    def _open_cells(self) -> list[Point]:
        cells = [
            (x, y)
            for y in range(5, 50, self.stride)
            for x in range(14, 60, self.stride)
            if (x, y) != self.start
        ]
        self._rng.shuffle(cells)
        return cells

    def _build_entities(self) -> None:
        cells = self._open_cells()
        self.entities: list[Entity] = []
        used = [self.start]

        def take() -> Point:
            for index in range(len(cells) - 1, -1, -1):
                candidate = cells[index]
                if all(
                    abs(candidate[0] - point[0]) + abs(candidate[1] - point[1])
                    >= self.stride * 3
                    for point in used
                ):
                    used.append(candidate)
                    return cells.pop(index)
            candidate = cells.pop()
            used.append(candidate)
            return candidate

        def add(kind: str, *, destination: Point | None = None) -> Entity:
            entity = Entity(kind, take(), next(self._entity_colors), destination)
            self.entities.append(entity)
            return entity

        if "resource" in self.spec.mechanics:
            add("resource")
        if "rotate" in self.spec.mechanics:
            add("rotate")
        if "recolor" in self.spec.mechanics:
            add("recolor", destination=(self.goal_signature.color, 0))
        if "key_gate" in self.spec.mechanics:
            add("key")
            self.gate_anchor = take()
        else:
            self.gate_anchor = None
        if "toggle" in self.spec.mechanics:
            add("toggle")
            self.toggle_wall = take()
        else:
            self.toggle_wall = None
        if "transport" in self.spec.mechanics:
            destination = take()
            add("transport", destination=destination)
        if "conveyor" in self.spec.mechanics:
            add("conveyor", destination=take())
        self.goal = Entity("goal", take(), self.avatar_color)
        self.entities.append(self.goal)
        for _ in range(self.spec.distractors):
            add("distractor")

        # Hazards are deliberately absent from the core curriculum.
        self.hazards: set[Point] = set()
        if "hazard" in self.spec.mechanics:
            self.hazards.update(take() for _ in range(2))

    def _build_transition_basin(self) -> None:
        """Generate an unrendered transition field across a naive goal path."""
        self.transition_basin: set[Point] = set()
        self.basin_destination = self.start
        if "transition_basin" not in self.spec.mechanics:
            return
        occupied = {entity.anchor for entity in self.entities}
        path: list[Point] = []
        cursor = self.start
        while cursor[0] != self.goal.anchor[0]:
            step = self.stride if self.goal.anchor[0] > cursor[0] else -self.stride
            cursor = (cursor[0] + step, cursor[1])
            path.append(cursor)
        while cursor[1] != self.goal.anchor[1]:
            step = self.stride if self.goal.anchor[1] > cursor[1] else -self.stride
            cursor = (cursor[0], cursor[1] + step)
            path.append(cursor)
        candidates = [
            point
            for point in path[:-1]
            if point not in occupied and point != self.start
        ]
        if not candidates:
            return
        center = candidates[len(candidates) // 2]
        horizontal_path = center[1] == self.start[1]
        offsets = (
            ((0, -self.stride), (0, 0), (0, self.stride))
            if horizontal_path
            else ((-self.stride, 0), (0, 0), (self.stride, 0))
        )
        for dx, dy in offsets:
            point = (center[0] + dx, center[1] + dy)
            if (
                14 <= point[0] <= 59
                and 5 <= point[1] <= 50
                and point not in occupied
            ):
                self.transition_basin.add(point)

    def reset(self) -> np.ndarray:
        self.avatar = self.start
        self.status = LatentSignature(self.avatar_color, self.initial_pattern)
        self.budget = min(43, self.spec.max_actions)
        self.key_collected = False
        self.toggle_open = False
        self.solved = False
        self.failed = False
        self.episode_actions = 0
        for entity in self.entities:
            entity.consumed = False
        return self.frame()

    def _blocked(self, point: Point) -> bool:
        x, y = point
        if not (14 <= x <= 59 and 5 <= y <= 50):
            return True
        if self.gate_anchor == point and not self.key_collected:
            return True
        return self.toggle_wall == point and not self.toggle_open

    def step(self, action: str) -> np.ndarray:
        if self.solved or self.failed:
            return self.frame()
        self.episode_actions += 1
        self.total_actions += 1
        self.budget -= 1
        dx, dy = self.action_deltas[action]
        destination = (self.avatar[0] + dx, self.avatar[1] + dy)
        if not self._blocked(destination):
            if destination in self.transition_basin:
                self.avatar = self.basin_destination
            else:
                self.avatar = destination
            if self.avatar in self.hazards:
                self.failed = True
            elif self.avatar == destination:
                self._interact(destination)
        if self.budget <= 0 and not self.solved:
            self.failed = True
        return self.frame()

    def _interact(self, point: Point) -> None:
        entity = next(
            (item for item in self.entities if not item.consumed and item.anchor == point),
            None,
        )
        if entity is None:
            return
        if entity.kind == "resource":
            self.budget = min(43, self.budget + 25)
            entity.consumed = True
        elif entity.kind == "rotate":
            self.status = self.status.rotate_clockwise()
        elif entity.kind == "recolor":
            assert entity.destination is not None
            self.status = LatentSignature(entity.destination[0], self.status.pattern)
        elif entity.kind == "transport":
            assert entity.destination is not None
            self.avatar = entity.destination
        elif entity.kind == "conveyor":
            assert entity.destination is not None
            self.avatar = entity.destination
            entity.consumed = True
        elif entity.kind == "key":
            self.key_collected = True
            entity.consumed = True
        elif entity.kind == "toggle":
            self.toggle_open = not self.toggle_open
        elif entity.kind == "goal":
            gate_ok = "key_gate" not in self.spec.mechanics or self.key_collected
            if gate_ok and self.status == self.goal_signature:
                self.solved = True

    @staticmethod
    def _draw_pattern(
        frame: np.ndarray, origin: Point, pattern: tuple[bool, ...], color: int
    ) -> None:
        ox, oy = origin
        for index, enabled in enumerate(pattern):
            if enabled:
                x = ox + (index % 3) * 2
                y = oy + (index // 3) * 2
                frame[y : y + 2, x : x + 2] = color

    @staticmethod
    def _draw_goal_pattern(
        frame: np.ndarray, origin: Point, pattern: tuple[bool, ...], color: int
    ) -> None:
        """Draw a sparse glyph so its enclosure remains the dominant color."""
        ox, oy = origin
        for index, enabled in enumerate(pattern):
            if enabled:
                frame[oy + 1 + index // 3, ox + 1 + index % 3] = color

    def frame(self) -> np.ndarray:
        frame = np.full((64, 64), self.floor, dtype=np.int16)
        # Interface: status glyph and an unambiguous remaining-budget run.
        frame[53:63, 1:11] = self.panel_background
        self._draw_pattern(frame, (3, 55), self.status.pattern, self.status.color)
        for x in range(12, 55):
            frame[61:63, x] = 6 + (x % 2)
        frame[61:63, 12 : 12 + min(43, self.budget)] = self.budget_color

        if self.gate_anchor is not None and not self.key_collected:
            x, y = self.gate_anchor
            frame[y : y + self.stride, x : x + self.stride] = self.wall
        if self.toggle_wall is not None and not self.toggle_open:
            x, y = self.toggle_wall
            frame[y : y + self.stride, x : x + self.stride] = self.wall
        for x, y in self.hazards:
            frame[y : y + self.stride, x : x + self.stride] = self.wall

        for entity in self.entities:
            if entity.consumed or entity.anchor == self.avatar:
                continue
            x, y = entity.anchor
            if entity.kind == "goal":
                frame[y : y + 5, x : x + 5] = self.panel_background
                self._draw_goal_pattern(
                    frame,
                    (x, y),
                    self.goal_signature.pattern,
                    self.goal_signature.color,
                )
                # A tiny current-state accent makes the target perceptually
                # comparable before a required recoloring, without encoding
                # the desired color as the current one.
                if self.avatar_color != self.goal_signature.color:
                    frame[y, x] = self.avatar_color
                    frame[y + 4, x + 4] = self.avatar_color
            else:
                frame[y : y + 5, x : x + 5] = entity.color

        x, y = self.avatar
        # A persistent outline carries object identity through transformations;
        # the interior exposes the changing latent color.
        frame[y : y + self.stride, x : x + self.stride] = 0
        frame[y + 1 : y + 4, x + 1 : x + 4] = self.status.color
        return frame


def run_controller(
    spec: SyntheticSpec,
    controller_factory: Callable[[list[str]], OntogenesisController] = OntogenesisController,
    *,
    max_resets: int = 2,
) -> SyntheticResult:
    env = SyntheticEnvironment(spec)
    controller = controller_factory(list(env.action_names))
    events: list[dict[str, object]] = []
    resets = 0
    while not env.solved and resets <= max_resets:
        while not env.solved and not env.failed:
            action = controller.choose(env.frame())
            env.step(action)
            events.extend(controller.drain_events())
        if env.solved:
            controller.mark_level_success()
            events.extend(controller.drain_events())
            break
        controller.mark_episode_failure()
        events.extend(controller.drain_events())
        resets += 1
        if resets <= max_resets:
            env.reset()
            controller.begin_episode()
    return SyntheticResult(
        name=spec.name,
        solved=env.solved,
        actions=env.total_actions,
        resets=min(resets, max_resets),
        events=tuple(events),
    )


def evaluate_split(
    split: str,
    controller_factory: Callable[[list[str]], OntogenesisController] = OntogenesisController,
) -> tuple[SyntheticResult, ...]:
    manifest = load_manifest()
    if split not in manifest:
        raise ValueError(f"unknown synthetic split {split!r}")
    return tuple(run_controller(spec, controller_factory) for spec in manifest[split])
