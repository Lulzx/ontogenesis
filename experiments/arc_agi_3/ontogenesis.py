"""A small, frame-only ontogenetic controller for ARC-AGI-3.

The controller deliberately knows no game IDs and no action semantics.  It
acquires translation laws from interventions, identifies a moving object from
the changed pixels, proposes rare visual objects as interaction targets, and
plans over the induced spatial quotient.  This is the first bounded rung, not
a general ARC-AGI-3 solver.
"""

from collections import Counter, deque
from dataclasses import dataclass
from typing import Any

import numpy as np

Point = tuple[int, int]
TileKey = tuple[int, ...]


@dataclass(frozen=True)
class AcquiredTranslation:
    action: str
    dx: int
    dy: int
    support: int


@dataclass(frozen=True)
class VisualTarget:
    anchor: Point
    kind: str
    colors: frozenset[int]
    size: int


@dataclass(frozen=True)
class LatentSignature:
    color: int
    pattern: tuple[bool, ...]

    def rotate_clockwise(self) -> "LatentSignature":
        grid = np.asarray(self.pattern, dtype=bool).reshape(3, 3)
        return LatentSignature(
            self.color, tuple(bool(v) for v in np.rot90(grid, -1).flat)
        )


@dataclass(frozen=True)
class StatusEdgeEffect:
    origin: Point
    action: str
    destination: Point
    before: LatentSignature
    after: LatentSignature


@dataclass(frozen=True)
class TransitionTileEffect:
    mode: str
    value: Point

    def resolve(self, expected: Point) -> Point:
        if self.mode == "absolute":
            return self.value
        return (expected[0] + self.value[0], expected[1] + self.value[1])


def _components(mask: np.ndarray) -> list[list[Point]]:
    height, width = mask.shape
    unseen = set(zip(*np.where(mask)[::-1]))
    out: list[list[Point]] = []
    while unseen:
        start = unseen.pop()
        todo = [start]
        component = [start]
        while todo:
            x, y = todo.pop()
            for point in ((x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)):
                px, py = point
                if 0 <= px < width and 0 <= py < height and point in unseen:
                    unseen.remove(point)
                    todo.append(point)
                    component.append(point)
        out.append(component)
    return out


def infer_translation(
    before: np.ndarray, after: np.ndarray, radius: int = 8
) -> tuple[int, int, int]:
    """Return the best frame-supported translation, with no action prior."""
    changed = before != after
    counts = Counter(int(v) for v in before.flat)
    rare = np.isin(before, [color for color, count in counts.items() if count <= 100])
    best = (0, 0, 0)
    height, width = before.shape
    for dy in range(-radius, radius + 1):
        for dx in range(-radius, radius + 1):
            if dx == 0 and dy == 0:
                continue
            x0, x1 = max(0, -dx), min(width, width - dx)
            y0, y1 = max(0, -dy), min(height, height - dy)
            src = before[y0:y1, x0:x1]
            dst = after[y0 + dy : y1 + dy, x0 + dx : x1 + dx]
            moved = (
                changed[y0:y1, x0:x1] & changed[y0 + dy : y1 + dy, x0 + dx : x1 + dx]
            )
            moved &= rare[y0:y1, x0:x1]
            support = int(np.count_nonzero((src == dst) & moved))
            candidate = (support, -abs(dx) - abs(dy), -abs(dy), dx, dy)
            incumbent = (
                best[2],
                -abs(best[0]) - abs(best[1]),
                -abs(best[1]),
                best[0],
                best[1],
            )
            if candidate > incumbent:
                best = (dx, dy, support)
    return best


class OntogenesisController:
    """Acquire a tiny executable ontology from one level's frame stream."""

    def __init__(
        self,
        action_names: list[str],
        inherited: "OntogenesisController | None" = None,
    ) -> None:
        self.action_names = list(action_names)
        self.translations: dict[str, AcquiredTranslation] = (
            dict(inherited.translations) if inherited is not None else {}
        )
        self.probes = deque(
            name for name in action_names if name not in self.translations
        )
        self.previous: np.ndarray | None = None
        self.last_action: str | None = None
        self.mover_anchor: Point | None = None
        self.initial_mover_anchor: Point | None = None
        self.mover_colors: frozenset[int] = (
            inherited.mover_colors if inherited is not None else frozenset()
        )
        self.initial_mover_colors: frozenset[int] = frozenset()
        self.stride = inherited.stride if inherited is not None else 1
        self.grid_phase: Point | None = (
            inherited.grid_phase if inherited is not None else None
        )
        self.floor_color: int | None = (
            inherited.floor_color if inherited is not None else None
        )
        self.targets: deque[VisualTarget] = deque()
        self.plan: deque[str] = deque()
        self.plan_is_product = False
        self.visited_targets: set[Point] = set()
        self.active_target: VisualTarget | None = None
        self.active_before: LatentSignature | None = None
        self.active_before_budget = 0
        self.modifier_effects: dict[Point, str] = {}
        self.resource_features: set[tuple[frozenset[int], int]] = (
            set(inherited.resource_features) if inherited is not None else set()
        )
        self.successful_target_features: set[tuple[frozenset[int], int]] = (
            set(inherited.successful_target_features)
            if inherited is not None
            else set()
        )
        self.resources_collected = 0
        self.goal_target: VisualTarget | None = None
        self.goal_signature_model: LatentSignature | None = None
        self.goal_attempt_signature: LatentSignature | None = None
        self.blocked_edges: set[tuple[Point, str]] = set()
        self.transition_edges: dict[tuple[Point, str], Point] = {}
        self.transition_entry_effects: dict[Point, Point] = {}
        self.transition_entry_conflicts: set[Point] = set()
        self.transition_tile_effects: dict[TileKey, TransitionTileEffect] = {}
        self.transition_tile_observations: dict[
            TileKey, dict[Point, Point]
        ] = {}
        self.transition_tile_conflicts: set[TileKey] = set()
        self.status_edge_effects: dict[tuple[Point, str], StatusEdgeEffect] = {}
        self.status_entry_effects: dict[Point, StatusEdgeEffect] = {}
        self.status_tile_effects: dict[TileKey, StatusEdgeEffect] = {}
        self.last_origin: Point | None = None
        self.last_was_planned = False
        self.last_was_transport = False
        self.last_was_expected_product_transport = False
        self.last_transport_target: VisualTarget | None = None
        self.active_target_pixels = 0
        self.failed_predictions = 0
        self.verified_interactions = 0
        self.action_attempts: Counter[str] = Counter()
        self.edge_attempts: Counter[tuple[Point, str]] = Counter()
        self.matched_detours_this_episode: set[
            tuple[frozenset[int], int]
        ] = set()
        self.failed_matched_detours: set[tuple[frozenset[int], int]] = set()
        self.events: deque[dict[str, Any]] = deque(maxlen=256)

    @staticmethod
    def _signature_record(signature: LatentSignature | None) -> dict[str, Any] | None:
        if signature is None:
            return None
        return {"color": signature.color, "pattern": signature.pattern}

    def drain_events(self) -> list[dict[str, Any]]:
        events = list(self.events)
        self.events.clear()
        return events

    def mark_level_success(self) -> None:
        """Bind the externally observed level transition to its causal option."""
        target = self.active_target or self.last_transport_target
        if target is not None:
            self.successful_target_features.add((target.colors, target.size))
            self.events.append(
                {
                    "event": "level_success",
                    "anchor": target.anchor,
                    "kind": target.kind,
                    "colors": sorted(target.colors),
                    "size": target.size,
                }
            )

    def mark_episode_failure(self) -> None:
        """Credit an externally observed failure to its edge and option trace."""
        self.failed_matched_detours.update(self.matched_detours_this_episode)
        if self.last_origin is None or self.last_action is None:
            return
        edge = (self.last_origin, self.last_action)
        edge_blocked = (
            self.last_was_planned
            and self.active_target is not None
            and not self.plan_is_product
        )
        if edge_blocked:
            self.blocked_edges.add(edge)
        self.plan.clear()
        self.plan_is_product = False
        self.events.append(
            {
                "event": (
                    "terminal_edge_blocked"
                    if edge_blocked
                    else "terminal_option_failed"
                ),
                "origin": self.last_origin,
                "action": self.last_action,
                "failed_matched_detours": len(self.failed_matched_detours),
            }
        )

    def begin_episode(self) -> None:
        """Forget pending motor state after a harness-managed level reset."""
        self.previous = None
        self.last_action = None
        self.last_origin = None
        self.mover_anchor = self.initial_mover_anchor
        if self.initial_mover_colors:
            self.mover_colors = self.initial_mover_colors
        self.last_was_planned = False
        self.last_was_transport = False
        self.last_was_expected_product_transport = False
        self.last_transport_target = None
        self.active_target = None
        self.active_before = None
        self.active_before_budget = 0
        self.active_target_pixels = 0
        self.resources_collected = 0
        self.goal_attempt_signature = None
        self.matched_detours_this_episode.clear()
        self.visited_targets.clear()
        self.plan.clear()
        self.plan_is_product = False
        self.targets.clear()

    def choose(self, frame: np.ndarray) -> str:
        frame = np.asarray(frame, dtype=np.int16)
        if frame.shape != (64, 64):
            raise ValueError(f"expected a 64x64 categorical frame, got {frame.shape}")

        if self.mover_anchor is None and self.mover_colors and self.translations:
            self._bootstrap_mover(frame)

        if self.previous is not None and self.last_action is not None:
            self.last_was_transport = False
            self.last_was_expected_product_transport = False
            self._learn_transition(self.previous, frame, self.last_action)
            self._validate_transition(frame)
            self._learn_status_edge(self.previous, frame)
        if self.last_transport_target is not None:
            self._finish_transport(self.last_transport_target)
            self.last_transport_target = None
        elif (
            self.last_was_transport
            and not self.last_was_expected_product_transport
            and self.active_target is not None
        ):
            # A transport may alter global/status pixels, but that evidence
            # belongs to the transition just crossed, not to a remote target
            # whose path happened to be interrupted.
            self.targets.appendleft(self.active_target)
            self.active_target = None
            self.active_before = None
            self.active_before_budget = 0
            self.active_target_pixels = 0
        if self.active_target is not None:
            if self._interaction_observed(frame):
                # Some targets fire on entry, before a leave-and-return plan
                # has been consumed.  The observation terminates that option.
                self.plan.clear()
                self._finish_target_visit(frame)
            elif not self.plan and self.last_was_planned:
                # A path ending is only a prediction.  Without contact or an
                # observable state change, retain the object hypothesis and
                # seek another route rather than hallucinating an interaction.
                self.targets.appendleft(self.active_target)
                self.active_target = None
                self.active_before = None
                self.active_before_budget = 0
                self.active_target_pixels = 0

        if self.probes:
            action = self.probes.popleft()
        else:
            if not self.targets:
                self.targets.extend(self._propose_targets(frame))
            if (
                not self.plan
                and not self._plan_goal_product_option(frame)
                and not self._plan_status_option(frame)
            ):
                self._plan_next_target(frame)
            action = self.plan.popleft() if self.plan else self._least_used_action()

        self.previous = frame.copy()
        self.last_origin = self.mover_anchor
        self.last_was_planned = bool(self.active_target is not None)
        self.last_action = action
        self.action_attempts[action] += 1
        if self.mover_anchor is not None:
            self.edge_attempts[(self.mover_anchor, action)] += 1
        return action

    def _learn_status_edge(self, before_frame: np.ndarray, frame: np.ndarray) -> None:
        if (
            self.last_origin is None
            or self.last_action is None
            or self.mover_anchor is None
        ):
            return
        before = self._status_signature(before_frame)
        after = self._status_signature(frame)
        if before is None or after is None or before == after:
            return
        effect = StatusEdgeEffect(
            self.last_origin,
            self.last_action,
            self.mover_anchor,
            before,
            after,
        )
        self.status_edge_effects[(effect.origin, effect.action)] = effect
        law = self.translations.get(effect.action)
        if law is not None:
            expected = (effect.origin[0] + law.dx, effect.origin[1] + law.dy)
            self.status_entry_effects[expected] = effect
            tile_key = self._tile_key(before_frame, expected)
            if tile_key is not None:
                self.status_tile_effects[tile_key] = effect
        # A changed latent state changes the value of every queued target.
        # Discard the old ordering so the next decision is ranked against the
        # new state rather than blindly continuing a now-obsolete option list.
        self.targets.clear()
        self.goal_attempt_signature = None
        self.events.append(
            {
                "event": "status_edge_learned",
                "origin": effect.origin,
                "action": effect.action,
                "destination": effect.destination,
                "before": self._signature_record(before),
                "after": self._signature_record(after),
            }
        )

    @staticmethod
    def _apply_status_effect(
        effect: StatusEdgeEffect, current: LatentSignature
    ) -> LatentSignature | None:
        before, after = effect.before, effect.after
        if before.color == after.color and before.rotate_clockwise() == after:
            if current.color == before.color:
                return current.rotate_clockwise()
        elif before.pattern == after.pattern and before.color != after.color:
            return LatentSignature(after.color, current.pattern)
        elif before.color == after.color and before.pattern != after.pattern:
            return LatentSignature(current.color, after.pattern)
        elif current == before:
            return after
        return None

    @staticmethod
    def _signature_distance(current: LatentSignature, goal: LatentSignature) -> int:
        color_cost = 9 if current.color != goal.color else 0
        pattern_cost = sum(a != b for a, b in zip(current.pattern, goal.pattern))
        return color_cost + pattern_cost

    @staticmethod
    def _status_effect_kind(
        before: LatentSignature | None, after: LatentSignature | None
    ) -> str:
        if before is None or after is None or before == after:
            return "none"
        if before.color != after.color and before.pattern == after.pattern:
            return "color"
        if before.color == after.color and before.rotate_clockwise() == after:
            return "rotation"
        return "shape"

    def _plan_status_option(self, frame: np.ndarray) -> bool:
        current = self._status_signature(frame)
        goal = self._goal_signature(frame)
        if current is None or goal is None or current == goal:
            return False
        current_distance = self._signature_distance(current, goal)
        choices: list[tuple[int, int, StatusEdgeEffect, list[str]]] = []
        for effect in self.status_edge_effects.values():
            if (effect.origin, effect.action) in self.blocked_edges:
                continue
            predicted = self._apply_status_effect(effect, current)
            if predicted is None:
                continue
            distance = self._signature_distance(predicted, goal)
            if distance >= current_distance:
                continue
            if self.mover_anchor == effect.origin:
                route: list[str] = []
            else:
                route = self._shortest_path(frame, effect.origin)
                if not route:
                    continue
            choices.append((distance, len(route), effect, route))
        if not choices:
            return False
        _, _, effect, route = min(
            choices,
            key=lambda item: (item[0], item[1], item[2].origin, item[2].action),
        )
        self.plan.extend([*route, effect.action])
        self.plan_is_product = False
        self.events.append(
            {
                "event": "status_option_planned",
                "origin": effect.origin,
                "action": effect.action,
                "path_length": len(route) + 1,
                "status": self._signature_record(current),
                "goal": self._signature_record(goal),
            }
        )
        return True

    def _plan_goal_product_option(self, frame: np.ndarray) -> bool:
        """Escalate to joint spatial/latent planning after negative evidence."""
        if not self.failed_matched_detours:
            return False
        current = self._status_signature(frame)
        goal = self._goal_signature(frame)
        if current is None or goal is None:
            return False
        choices: list[tuple[int, Point, VisualTarget, list[str]]] = []
        for target in self.targets:
            if target.kind != "goal_analogue":
                continue
            if self.goal_attempt_signature is not None and current == self.goal_attempt_signature:
                continue
            route = self._shortest_product_path(frame, target.anchor, current, goal)
            if route and len(route) <= self._action_budget(frame):
                choices.append((len(route), target.anchor, target, route))
        if not choices:
            return False
        _, _, target, route = min(choices, key=lambda item: (item[0], item[1]))
        self.plan.extend(route)
        self.plan_is_product = True
        self.active_target = target
        self.active_before = current
        self.active_before_budget = self._action_budget(frame)
        self.active_target_pixels = self._target_pixel_count(frame, target)
        self.events.append(
            {
                "event": "product_goal_planned",
                "anchor": target.anchor,
                "path_length": len(route),
                "status": self._signature_record(current),
                "goal": self._signature_record(goal),
            }
        )
        return True

    def _validate_transition(self, frame: np.ndarray) -> None:
        if self.last_origin is None or self.last_action is None:
            return
        law = self.translations.get(self.last_action)
        if law is None or self.mover_anchor is None:
            return
        expected = (self.last_origin[0] + law.dx, self.last_origin[1] + law.dy)
        edge = (self.last_origin, self.last_action)
        if self.mover_anchor == expected:
            self.blocked_edges.discard(edge)
            return
        if self.mover_anchor == self.last_origin:
            self.blocked_edges.add(edge)
            self.failed_predictions += 1
            if self.last_was_planned:
                self.plan.clear()
            return
        # A reproducible nonlocal outcome is a newly acquired transition
        # primitive.  It is neither failed motion nor an object mutation.
        known_destination = self.transition_edges.get(edge)
        self.transition_edges[edge] = self.mover_anchor
        if expected not in self.transition_entry_conflicts:
            entry_destination = self.transition_entry_effects.get(expected)
            if entry_destination is not None and entry_destination != self.mover_anchor:
                self.transition_entry_effects.pop(expected, None)
                self.transition_entry_conflicts.add(expected)
            else:
                self.transition_entry_effects[expected] = self.mover_anchor
        tile_key = self._tile_key(self.previous, expected)
        if tile_key is not None:
            self._learn_tile_transition(tile_key, expected, self.mover_anchor)
        self.events.append(
            {
                "event": "nonlocal_transition",
                "origin": self.last_origin,
                "action": self.last_action,
                "destination": self.mover_anchor,
            }
        )
        self.last_was_transport = True
        self.last_was_expected_product_transport = (
            self.plan_is_product and known_destination == self.mover_anchor
        )
        self.failed_predictions += 1
        if self.last_was_planned and not self.last_was_expected_product_transport:
            self.plan.clear()
            self.plan_is_product = False
            target = self.active_target
            if target is not None and expected == target.anchor:
                self.last_transport_target = target

    def _interaction_observed(self, frame: np.ndarray) -> bool:
        target = self.active_target
        if target is None:
            return False
        status_changed = self._status_signature(frame) != self.active_before
        previous_budget = (
            self._action_budget(self.previous)
            if self.previous is not None
            else self.active_before_budget
        )
        budget_increased = self._action_budget(frame) > previous_budget
        remaining = self._target_pixel_count(frame, target)
        target_changed = target.kind == "resource" and remaining < max(
            1, self.active_target_pixels // 2
        )
        contact = self.mover_anchor == target.anchor
        # Remote animation and HUD changes are not object interactions.  A
        # non-contact effect is admissible only when the planned option has
        # actually reached its endpoint.
        return contact or (
            not self.plan and (status_changed or budget_increased or target_changed)
        )

    def _finish_transport(self, target: VisualTarget) -> None:
        self.visited_targets.add(target.anchor)
        self.modifier_effects[target.anchor] = "transport"
        self.active_target = None
        self.active_before = None
        self.active_before_budget = 0
        self.active_target_pixels = 0
        self.plan.clear()
        self.targets.clear()
        self.verified_interactions += 1
        self.events.append(
            {
                "event": "interaction",
                "anchor": target.anchor,
                "kind": target.kind,
                "colors": sorted(target.colors),
                "size": target.size,
                "effect": "transport",
            }
        )

    def _target_pixel_count(self, frame: np.ndarray, target: VisualTarget) -> int:
        x, y = target.anchor
        radius = max(2, self.stride)
        crop = frame[
            max(0, y - radius) : min(53, y + self.stride + radius),
            max(0, x - radius) : min(64, x + self.stride + radius),
        ]
        return int(np.count_nonzero(np.isin(crop, list(target.colors))))

    def _remote_scene_changed(
        self, before: np.ndarray | None, after: np.ndarray, target: VisualTarget
    ) -> bool:
        """Observe a nonlocal mutation without assigning it a mechanic label."""
        if before is None:
            return False
        changed = before != after
        changed[53:, :] = False
        radius = max(1, self.stride)
        for anchor in (self.last_origin, target.anchor):
            if anchor is None:
                continue
            x, y = anchor
            changed[
                max(0, y - 1) : min(53, y + radius + 1),
                max(0, x - 1) : min(64, x + radius + 1),
            ] = False
        return int(np.count_nonzero(changed)) >= 4

    def _bootstrap_mover(self, frame: np.ndarray) -> None:
        components = [
            component
            for component in _components(np.isin(frame, list(self.mover_colors)))
            if max(y for _, y in component) < 53
        ]
        if not components:
            return
        expected = self.stride * self.stride
        component = min(
            components,
            key=lambda c: (abs(len(c) - expected), -min(y for _, y in c)),
        )
        self.mover_anchor = self._snap_anchor(
            (int(min(x for x, _ in component)), int(min(y for _, y in component)))
        )
        self.mover_colors = frozenset(int(frame[y, x]) for x, y in component)
        if self.initial_mover_anchor is None:
            self.initial_mover_anchor = self.mover_anchor
            self.initial_mover_colors = self.mover_colors
        x, y = self.mover_anchor
        ring = frame[
            max(0, y - 1) : min(64, y + self.stride + 1),
            max(0, x - 1) : min(64, x + self.stride + 1),
        ]
        surrounding = [int(v) for v in ring.flat if int(v) not in self.mover_colors]
        if surrounding and self.floor_color is None:
            self.floor_color = Counter(surrounding).most_common(1)[0][0]

    def _learn_transition(
        self, before: np.ndarray, after: np.ndarray, action: str
    ) -> None:
        frozen = self.translations.get(action)
        if frozen is None:
            dx, dy, support = infer_translation(before, after)
            if support < 4:
                return
            law = AcquiredTranslation(action, dx, dy, support)
            self.translations[action] = law
            nonzero = [abs(v) for v in (dx, dy) if v]
            if nonzero:
                self.stride = min(nonzero)
        else:
            dx, dy, support = frozen.dx, frozen.dy, frozen.support

        if self.mover_colors and self.mover_anchor is not None:
            origin = self.mover_anchor
            expected = (origin[0] + dx, origin[1] + dy)
            template = before[
                origin[1] : origin[1] + self.stride,
                origin[0] : origin[0] + self.stride,
            ]

            def support(anchor: Point) -> int:
                x, y = anchor
                if (
                    x < 0
                    or y < 0
                    or x + self.stride > after.shape[1]
                    or y + self.stride > after.shape[0]
                ):
                    return -1
                tile = after[y : y + self.stride, x : x + self.stride]
                mover_mask = np.isin(template, list(self.mover_colors))
                return int(np.count_nonzero((tile == template) & mover_mask))

            # A failed intervention leaves the mover at its origin.  Compare
            # only the two causal hypotheses before consulting lookalike
            # components elsewhere in the scene.
            old_support = support(origin)
            expected_support = support(expected)
            threshold = max(2, len(self.mover_colors))
            if old_support >= expected_support and old_support >= threshold:
                self.mover_anchor = origin
            elif expected_support >= threshold:
                self.mover_anchor = expected
            else:
                before_components = _components(
                    np.isin(before[:53], list(self.mover_colors))
                )
                source = min(
                    before_components,
                    key=lambda c: min(
                        (px - origin[0]) ** 2 + (py - origin[1]) ** 2 for px, py in c
                    ),
                    default=[],
                )
                source_hist = Counter(int(before[py, px]) for px, py in source)
                source_width = (
                    max(px for px, _ in source) - min(px for px, _ in source) + 1
                    if source
                    else self.stride
                )
                source_height = (
                    max(py for _, py in source) - min(py for _, py in source) + 1
                    if source
                    else self.stride
                )
                components = _components(np.isin(after[:53], list(self.mover_colors)))
                if components:

                    def identity_distance(component: list[Point]) -> tuple[int, int]:
                        hist = Counter(int(after[py, px]) for px, py in component)
                        width = (
                            max(px for px, _ in component)
                            - min(px for px, _ in component)
                            + 1
                        )
                        height = (
                            max(py for _, py in component)
                            - min(py for _, py in component)
                            + 1
                        )
                        colors = set(source_hist) | set(hist)
                        signature_error = (
                            abs(len(component) - len(source)) * 4
                            + abs(width - source_width) * 2
                            + abs(height - source_height) * 2
                            + sum(
                                abs(hist[color] - source_hist[color])
                                for color in colors
                            )
                        )
                        spatial_tiebreak = min(
                            (np.mean([p[0] for p in component]) - origin[0]) ** 2
                            + (np.mean([p[1] for p in component]) - origin[1]) ** 2,
                            (np.mean([p[0] for p in component]) - expected[0]) ** 2
                            + (np.mean([p[1] for p in component]) - expected[1]) ** 2,
                        )
                        return signature_error, int(spatial_tiebreak)

                    component = min(
                        components,
                        key=identity_distance,
                    )
                    self.mover_anchor = self._snap_anchor(
                        (
                            int(min(x for x, _ in component)),
                            int(min(y for _, y in component)),
                        )
                    )
            return

        changed = before != after
        height, width = before.shape
        x0, x1 = max(0, -dx), min(width, width - dx)
        y0, y1 = max(0, -dy), min(height, height - dy)
        match = before[y0:y1, x0:x1] == after[y0 + dy : y1 + dy, x0 + dx : x1 + dx]
        counts = Counter(int(v) for v in before.flat)
        rare_values = [color for color, count in counts.items() if count <= 100]
        match &= changed[y0:y1, x0:x1] & changed[y0 + dy : y1 + dy, x0 + dx : x1 + dx]
        match &= np.isin(before[y0:y1, x0:x1], rare_values)
        ys, xs = np.where(match)
        if not len(xs):
            return
        source_points = [(int(x + x0), int(y + y0)) for y, x in zip(ys, xs)]
        palette = frozenset(int(before[y, x]) for x, y in source_points)
        components = _components(np.isin(after, list(palette)))
        predicted = (np.mean(xs + x0) + dx, np.mean(ys + y0) + dy)
        component = min(
            components,
            key=lambda c: (
                (np.mean([p[0] for p in c]) - predicted[0]) ** 2
                + (np.mean([p[1] for p in c]) - predicted[1]) ** 2
            ),
        )
        self.mover_colors = frozenset(int(after[y, x]) for x, y in component)
        raw_anchor = (
            int(min(x for x, _ in component)),
            int(min(y for _, y in component)),
        )
        if self.grid_phase is None:
            self.grid_phase = (
                raw_anchor[0] % self.stride,
                raw_anchor[1] % self.stride,
            )
        self.mover_anchor = self._snap_anchor(raw_anchor)
        if self.initial_mover_anchor is None:
            self.initial_mover_anchor = self.mover_anchor
            self.initial_mover_colors = self.mover_colors

        vacated = []
        for x, y in source_points:
            if (
                0 <= x < width
                and 0 <= y < height
                and int(after[y, x]) not in self.mover_colors
            ):
                vacated.append(int(after[y, x]))
        if vacated:
            self.floor_color = Counter(vacated).most_common(1)[0][0]

    @staticmethod
    def _pattern(mask: np.ndarray) -> tuple[bool, ...] | None:
        ys, xs = np.where(mask)
        if not len(xs):
            return None
        min_x, max_x = int(xs.min()), int(xs.max())
        min_y, max_y = int(ys.min()), int(ys.max())
        width, height = max_x - min_x + 1, max_y - min_y + 1
        normalized = np.zeros((3, 3), dtype=bool)
        for x, y in zip(xs, ys):
            nx = min(2, (int(x) - min_x) * 3 // width)
            ny = min(2, (int(y) - min_y) * 3 // height)
            normalized[ny, nx] = True
        return tuple(bool(v) for v in normalized.flat)

    def _snap_anchor(self, anchor: Point) -> Point:
        if self.grid_phase is None or self.stride <= 0:
            return anchor
        phase_x, phase_y = self.grid_phase
        return (
            phase_x + round((anchor[0] - phase_x) / self.stride) * self.stride,
            phase_y + round((anchor[1] - phase_y) / self.stride) * self.stride,
        )

    def _status_signature(self, frame: np.ndarray) -> LatentSignature | None:
        # ARC's frame reserves the lower edge for interface information.  Find
        # the non-background glyph in the left status panel.
        return self._panel_signature(frame[53:63, 1:11])

    def _panel_signature(self, roi: np.ndarray) -> LatentSignature | None:
        counts = Counter(int(v) for v in roi.flat)
        if len(counts) < 2:
            return None
        background = counts.most_common(1)[0][0]
        foreground = max(
            ((count, color) for color, count in counts.items() if color != background),
            default=None,
        )
        if foreground is None or foreground[0] < 2:
            return None
        color = foreground[1]
        pattern = self._pattern(roi == color)
        return None if pattern is None else LatentSignature(color, pattern)

    def _goal_signature(
        self, frame: np.ndarray, target: VisualTarget | None = None
    ) -> LatentSignature | None:
        if self.goal_signature_model is not None:
            return self.goal_signature_model
        target = target or self.goal_target
        if target is None:
            return None
        x, y = target.anchor
        x0, y0 = max(0, x - 2), max(0, y - 2)
        crop = frame[
            y0 : min(64, y + self.stride + 2), x0 : min(64, x + self.stride + 2)
        ]
        choices = Counter(
            int(value) for value in crop.flat if int(value) in target.colors
        )
        if not choices:
            return None
        if len(choices) == 1:
            color = next(iter(choices))
        else:
            enclosure = choices.most_common(1)[0][0]
            foreground = [
                (count, color)
                for color, count in choices.items()
                if color != enclosure and count >= 2
            ]
            if not foreground:
                return None
            _, color = max(foreground)
        pattern = self._pattern(crop == color)
        return None if pattern is None else LatentSignature(color, pattern)

    @staticmethod
    def _action_budget(frame: np.ndarray) -> int:
        """Read the longest lower-interface bar without assuming its color."""
        best = 0
        for row in frame[61:63, 12:55]:
            run = 0
            previous: int | None = None
            for raw in row:
                color = int(raw)
                if color == previous:
                    run += 1
                else:
                    previous, run = color, 1
                best = max(best, run)
        return best

    def _finish_target_visit(self, frame: np.ndarray) -> None:
        assert self.active_target is not None
        target = self.active_target
        before = (
            self._status_signature(self.previous)
            if self.previous is not None
            else self.active_before
        )
        before_budget = (
            self._action_budget(self.previous)
            if self.previous is not None
            else self.active_before_budget
        )
        after = self._status_signature(frame)
        after_budget = self._action_budget(frame)
        # Contact can change every visible feature of the mover.  Identity is
        # temporal (the endpoint of the verified intervention), not a frozen
        # color label.  Re-anchor at the contacted object and acquire the new
        # appearance before looking for the next object.
        self.mover_anchor = target.anchor
        x, y = target.anchor
        tile = frame[y : y + self.stride, x : x + self.stride]
        transformed_colors = frozenset(
            int(value)
            for value in tile.flat
            if self.floor_color is None or int(value) != self.floor_color
        )
        if transformed_colors:
            self.mover_colors = transformed_colors
        self.visited_targets.add(target.anchor)
        repeat_modifier = False
        effect = "none"
        status_effect = self._status_effect_kind(before, after)
        if status_effect != "none":
            effect = status_effect
            self.modifier_effects[target.anchor] = effect
            goal = self._goal_signature(frame)
            if effect == "rotation":
                repeat_modifier = self._rotation_distance(after, goal) > 0
            elif effect == "color":
                repeat_modifier = (
                    after is not None
                    and goal is not None
                    and after.color != goal.color
                )
            elif effect == "shape":
                repeat_modifier = (
                    after is not None
                    and goal is not None
                    and after.pattern != goal.pattern
                )
        elif after_budget > before_budget or target.kind == "resource":
            effect = "resource"
            self.modifier_effects[target.anchor] = effect
            self.resource_features.add((target.colors, target.size))
            self.resources_collected += 1
        elif target.kind != "resource":
            if effect == "none" and target.kind == "goal_analogue":
                self.goal_attempt_signature = after
            else:
                self.modifier_effects[target.anchor] = effect
        remote_changed = self._remote_scene_changed(self.previous, frame, target)
        if remote_changed:
            # Only invalidate a negative observation made in the old scene.
            # Do not promote or prioritize a new mechanic from one transition.
            self.goal_attempt_signature = None
        self.active_target = None
        self.active_before = None
        self.active_before_budget = 0
        self.active_target_pixels = 0
        self.verified_interactions += 1
        self.events.append(
            {
                "event": "interaction",
                "anchor": target.anchor,
                "kind": target.kind,
                "colors": sorted(target.colors),
                "size": target.size,
                "effect": effect,
                "before": self._signature_record(before),
                "after": self._signature_record(after),
                "budget_before": before_budget,
                "budget_after": after_budget,
                "remote_changed": remote_changed,
            }
        )
        self.targets.clear()
        if repeat_modifier:
            # The mover occludes a modifier while standing on it, so retain the
            # causal object explicitly and plan a leave-and-return experiment.
            self.targets.append(target)

    def _propose_targets(self, frame: np.ndarray) -> list[VisualTarget]:
        if self.mover_anchor is None or not self.mover_colors:
            return []
        counts = Counter(int(v) for v in frame.flat)
        rare = {color for color, count in counts.items() if count <= 100}
        components = _components(np.isin(frame, list(rare)))
        if self.grid_phase is None:
            self.grid_phase = (
                self.mover_anchor[0] % self.stride,
                self.mover_anchor[1] % self.stride,
            )
        phase_x, phase_y = self.grid_phase
        candidates: dict[Point, VisualTarget] = {}
        for component in components:
            if not (2 <= len(component) <= 50):
                continue
            min_x, min_y = min(x for x, _ in component), min(y for _, y in component)
            min_component_y = min(y for _, y in component)
            # The public format reserves edge bands for interface chrome.  This
            # is a format prior, not a game-specific coordinate.
            if min_x < 12 or min_component_y >= 53:
                continue
            anchor = (
                phase_x + round((min_x - phase_x) / self.stride) * self.stride,
                phase_y + round((min_y - phase_y) / self.stride) * self.stride,
            )
            if anchor == self.mover_anchor:
                continue
            colors = frozenset(int(frame[y, x]) for x, y in component)
            panel_background = Counter(
                int(value) for value in frame[53:63, 1:11].flat
            ).most_common(1)[0][0]
            feature = (colors, len(component))
            if feature in self.successful_target_features or (
                panel_background in colors and not colors.isdisjoint(self.mover_colors)
            ):
                kind = "goal_analogue"
            elif feature in self.resource_features:
                kind = "resource"
            else:
                kind = "state_modifier"
            target = VisualTarget(anchor, kind, colors, len(component))
            old = candidates.get(anchor)
            if old is None or target.size > old.size:
                candidates[anchor] = target

        for target in candidates.values():
            if target.kind == "goal_analogue":
                self.goal_target = target
                signature = self._goal_signature(frame, target)
                if signature is not None:
                    self.goal_signature_model = signature
                break
        origin = self.mover_anchor
        current = self._status_signature(frame)
        goal = self._goal_signature(frame)
        resources_available = any(
            target.kind == "resource" for target in candidates.values()
        )
        desired_paths = [
            len(path)
            for target in candidates.values()
            if target.kind == "state_modifier"
            and self.modifier_effects.get(target.anchor) != "none"
            and (path := self._shortest_path(frame, target.anchor))
        ]
        budget = self._action_budget(frame)
        need_resource = (
            resources_available
            and bool(desired_paths)
            and min(desired_paths) * 2 >= budget
        )

        def rank(target: VisualTarget) -> tuple[int, int, int]:
            effect = self.modifier_effects.get(target.anchor)
            matched = current is not None and current == goal
            feature = (target.colors, target.size)
            failed_matched_detour = matched and feature in self.failed_matched_detours
            if effect == "transport":
                priority = 0 if matched and not failed_matched_detour else 5
            elif (
                effect == "rotation"
                and self._rotation_distance(current, goal) > 0
                or (
                    effect == "color"
                    and current is not None
                    and goal is not None
                    and current.color != goal.color
                )
                or (
                    effect == "shape"
                    and current is not None
                    and goal is not None
                    and current.pattern != goal.pattern
                )
            ):
                priority = 0
            elif target.kind == "goal_analogue":
                priority = 1 if matched else 3
            elif target.kind == "resource":
                if failed_matched_detour:
                    priority = 2
                else:
                    priority = (
                        0
                        if self.resources_collected == 0 or matched or need_resource
                        else 2
                    )
            elif effect == "none":
                priority = 5
            elif effect is None:
                priority = 1
            else:
                priority = 4
            distance = abs(target.anchor[0] - origin[0]) + abs(
                target.anchor[1] - origin[1]
            )
            return (priority, distance, -target.size)

        return sorted(
            candidates.values(),
            key=rank,
        )

    @staticmethod
    def _rotation_distance(
        current: LatentSignature | None, goal: LatentSignature | None
    ) -> int:
        if current is None or goal is None or current.color != goal.color:
            return 0
        rotated = current
        for turns in range(4):
            if rotated == goal:
                return turns
            rotated = rotated.rotate_clockwise()
        return 0

    def _plan_next_target(self, frame: np.ndarray) -> None:
        deferred: list[VisualTarget] = []
        for _ in range(len(self.targets)):
            target = self.targets.popleft()
            if self.modifier_effects.get(target.anchor) == "none":
                continue
            current = self._status_signature(frame)
            if (
                target.kind == "goal_analogue"
                and self.goal_attempt_signature is not None
                and current == self.goal_attempt_signature
            ):
                deferred.append(target)
                continue
            path = self._shortest_path(frame, target.anchor)
            if path:
                current = self._status_signature(frame)
                goal = self._goal_signature(frame)
                if (
                    current is not None
                    and current == goal
                    and (
                        target.kind == "resource"
                        or self.modifier_effects.get(target.anchor) == "transport"
                    )
                ):
                    self.matched_detours_this_episode.add(
                        (target.colors, target.size)
                    )
                self.plan.extend(path)
                self.plan_is_product = False
                self.active_target = target
                self.active_before = current
                self.active_before_budget = self._action_budget(frame)
                self.active_target_pixels = self._target_pixel_count(frame, target)
                self.targets.extend(deferred)
                self.events.append(
                    {
                        "event": "option_planned",
                        "anchor": target.anchor,
                        "kind": target.kind,
                        "colors": sorted(target.colors),
                        "size": target.size,
                        "known_effect": self.modifier_effects.get(target.anchor),
                        "path_length": len(path),
                        "status": self._signature_record(current),
                        "goal": self._signature_record(self._goal_signature(frame)),
                    }
                )
                return
            deferred.append(target)
        self.targets.extend(deferred)

    def _shortest_path(self, frame: np.ndarray, goal: Point) -> list[str]:
        if self.mover_anchor is None or self.floor_color is None:
            return []
        by_delta = {(law.dx, law.dy): law.action for law in self.translations.values()}
        moves = [delta for delta in by_delta if delta != (0, 0)]
        if not moves:
            return []
        start = self.mover_anchor
        if start == goal:
            for dx, dy in moves:
                inverse = by_delta.get((-dx, -dy))
                neighbor = (start[0] + dx, start[1] + dy)
                if inverse is not None and self._traversable(frame, neighbor, goal):
                    return [by_delta[(dx, dy)], inverse]
            return []
        queue = deque([start])
        parent: dict[Point, tuple[Point, str] | None] = {start: None}
        while queue:
            point = queue.popleft()
            if point == goal:
                break
            for delta in moves:
                action = by_delta[delta]
                expected = (point[0] + delta[0], point[1] + delta[1])
                edge = (point, action)
                tile_key = self._tile_key(frame, expected)
                nxt = self.transition_edges.get(edge)
                if nxt is None:
                    nxt = self.transition_entry_effects.get(expected)
                if nxt is None and tile_key is not None:
                    schema = self.transition_tile_effects.get(tile_key)
                    if schema is not None:
                        nxt = schema.resolve(expected)
                if nxt is None:
                    nxt = expected
                if (
                    nxt in parent
                    or edge in self.blocked_edges
                    or not self._traversable(frame, nxt, goal)
                ):
                    continue
                parent[nxt] = (point, action)
                queue.append(nxt)
        if goal not in parent:
            return []
        actions: list[str] = []
        cursor = goal
        while parent[cursor] is not None:
            previous, action = parent[cursor]
            actions.append(action)
            cursor = previous
        actions.reverse()
        return actions

    def _shortest_product_path(
        self,
        frame: np.ndarray,
        goal_position: Point,
        start_status: LatentSignature,
        goal_status: LatentSignature,
    ) -> list[str]:
        """Plan over learned position and latent-state transitions together."""
        if self.mover_anchor is None or self.floor_color is None:
            return []
        by_delta = {(law.dx, law.dy): law.action for law in self.translations.values()}
        moves = [delta for delta in by_delta if delta != (0, 0)]
        if not moves:
            return []
        start = (self.mover_anchor, start_status)
        queue = deque([start])
        parent: dict[
            tuple[Point, LatentSignature],
            tuple[tuple[Point, LatentSignature], str] | None,
        ] = {start: None}
        finish: tuple[Point, LatentSignature] | None = None
        while queue:
            position, status = queue.popleft()
            state = (position, status)
            if position == goal_position and status == goal_status:
                finish = state
                break
            for delta in moves:
                action = by_delta[delta]
                edge = (position, action)
                if edge in self.blocked_edges:
                    continue
                expected = (position[0] + delta[0], position[1] + delta[1])
                tile_key = self._tile_key(frame, expected)
                nxt = self.transition_edges.get(edge)
                if nxt is None:
                    nxt = self.transition_entry_effects.get(expected)
                if nxt is None and tile_key is not None:
                    schema = self.transition_tile_effects.get(tile_key)
                    if schema is not None:
                        nxt = schema.resolve(expected)
                if nxt is None:
                    nxt = expected
                learned_interaction = (
                    edge in self.status_edge_effects
                    or edge in self.transition_edges
                    or expected in self.status_entry_effects
                    or expected in self.transition_entry_effects
                    or (
                        tile_key is not None
                        and (
                            tile_key in self.status_tile_effects
                            or tile_key in self.transition_tile_effects
                        )
                    )
                )
                if (
                    not learned_interaction
                    and not self._traversable(frame, nxt, goal_position)
                ):
                    continue
                next_status = status
                effect = self.status_edge_effects.get(edge)
                if effect is None:
                    effect = self.status_entry_effects.get(expected)
                if effect is None and tile_key is not None:
                    effect = self.status_tile_effects.get(tile_key)
                if effect is not None:
                    predicted = self._apply_status_effect(effect, status)
                    if predicted is None:
                        continue
                    next_status = predicted
                next_state = (nxt, next_status)
                if next_state in parent:
                    continue
                parent[next_state] = (state, action)
                queue.append(next_state)
        if finish is None:
            return []
        actions: list[str] = []
        cursor = finish
        while parent[cursor] is not None:
            previous, action = parent[cursor]
            actions.append(action)
            cursor = previous
        actions.reverse()
        return actions

    def _tile_key(self, frame: np.ndarray | None, anchor: Point) -> TileKey | None:
        """Return a visual interaction schema, excluding ordinary floor."""
        if frame is None or self.floor_color is None:
            return None
        x, y = anchor
        if x < 0 or y < 0 or x + self.stride > 64 or y + self.stride > 60:
            return None
        tile = frame[y : y + self.stride, x : x + self.stride]
        if int(np.count_nonzero(tile != self.floor_color)) < 2:
            return None
        return tuple(int(value) for value in tile.flat)

    def _learn_tile_transition(
        self, tile_key: TileKey, expected: Point, observed: Point
    ) -> None:
        if tile_key in self.transition_tile_conflicts:
            return
        observations = self.transition_tile_observations.setdefault(tile_key, {})
        observations[expected] = observed
        if len(observations) < 2:
            return
        destinations = set(observations.values())
        displacements = {
            (destination[0] - origin[0], destination[1] - origin[1])
            for origin, destination in observations.items()
        }
        if len(destinations) == 1:
            self.transition_tile_effects[tile_key] = TransitionTileEffect(
                "absolute", next(iter(destinations))
            )
        elif len(displacements) == 1:
            self.transition_tile_effects[tile_key] = TransitionTileEffect(
                "relative", next(iter(displacements))
            )
        else:
            self.transition_tile_effects.pop(tile_key, None)
            self.transition_tile_conflicts.add(tile_key)

    def _traversable(self, frame: np.ndarray, anchor: Point, goal: Point) -> bool:
        x, y = anchor
        if x < 0 or y < 0 or x + self.stride > 64 or y + self.stride > 60:
            return False
        # Goal sprites can visually occupy both the destination cell and its
        # approach cell.  Treat the final one-stride neighborhood as an
        # interaction surface; the environment remains the verifier.
        if (
            anchor == self.mover_anchor
            or abs(anchor[0] - goal[0]) + abs(anchor[1] - goal[1]) <= self.stride
        ):
            return True
        tile = frame[y : y + self.stride, x : x + self.stride]
        allowed = np.isin(tile, [self.floor_color, *self.mover_colors])
        return int(np.count_nonzero(allowed)) >= max(1, tile.size - 2)

    def _least_used_action(self) -> str:
        # Prefer an untested intervention in the current observed state. This
        # turns fallback into active causal exploration rather than a repeated
        # lexicographic action at a wall or model boundary.
        origin = self.mover_anchor
        return min(
            self.action_names,
            key=lambda name: (
                self.edge_attempts[(origin, name)] if origin is not None else 0,
                self.action_attempts[name],
                name,
            ),
        )

    def machine_record(self) -> str:
        laws = ",".join(
            f"{name}:{law.dx}:{law.dy}:{law.support}"
            for name, law in sorted(self.translations.items())
        )
        return (
            "experiment=arc_agi_3_ontogenesis,"
            f"translations=[{laws}],stride={self.stride},"
            f"grid_phase={self.grid_phase},"
            f"mover_anchor={self.mover_anchor},floor={self.floor_color},"
            f"targets_visited={len(self.visited_targets)},"
            f"modifier_effects={sorted(self.modifier_effects.items())},"
            f"blocked_edges={len(self.blocked_edges)},"
            f"transition_edges={len(self.transition_edges)},"
            f"status_edges={len(self.status_edge_effects)},"
            f"failed_predictions={self.failed_predictions},"
            f"verified_interactions={self.verified_interactions},"
            f"unique_interventions={len(self.edge_attempts)},"
            f"success_prototypes={len(self.successful_target_features)},"
            "game_specific_rules=false,frame_only=true"
        )


# The official Kaggle starter packages exactly one file as agent/my_agent.py.
# Keeping the adapter here makes this module directly copyable while leaving
# its inference core importable without the competition framework installed.
try:  # pragma: no cover - exercised by the official framework smoke test
    from agents.agent import Agent
    from arcengine import FrameData, GameAction, GameState
except ImportError:  # Unit tests need only NumPy and the inference core.
    FrameData = GameAction = GameState = Agent = None  # type: ignore[assignment]


if Agent is not None:

    class MyAgent(Agent):  # type: ignore[misc, valid-type]
        """Official-harness adapter for the frame-only online learner."""

        MAX_ACTIONS = 1000

        def __init__(self, *args: Any, **kwargs: Any) -> None:
            super().__init__(*args, **kwargs)
            self.controller: OntogenesisController | None = None
            self.completed_level = 0
            self.awaiting_reset = False
            self.clicks: deque[Point] = deque()
            self.clicked: set[Point] = set()

        @property
        def name(self) -> str:
            return f"{super().name}.{self.MAX_ACTIONS}"

        def is_done(self, frames: list[FrameData], latest_frame: FrameData) -> bool:
            return latest_frame.state is GameState.WIN

        @staticmethod
        def _available(latest_frame: FrameData) -> list[GameAction]:
            return [
                GameAction.from_id(action_id)
                for action_id in latest_frame.available_actions
            ]

        def _click_action(self, frame: np.ndarray) -> GameAction:
            if not self.clicks:
                counts = Counter(int(value) for value in frame.flat)
                background = counts.most_common(1)[0][0]
                rare_colors = {
                    color
                    for color, count in counts.items()
                    if color != background and count <= 100
                }
                components = _components(np.isin(frame, list(rare_colors)))
                proposed: list[tuple[int, Point]] = []
                for component in components:
                    x = round(sum(px for px, _ in component) / len(component))
                    y = round(sum(py for _, py in component) / len(component))
                    proposed.append((-len(component), (x, y)))
                proposed.sort(key=lambda item: (item[0], item[1][1], item[1][0]))
                self.clicks.extend(
                    point for _, point in proposed if point not in self.clicked
                )
            if self.clicks:
                point = self.clicks.popleft()
            else:
                # Deterministic coverage is a last resort, not a hidden rule.
                index = len(self.clicked)
                point = ((index % 8) * 8 + 4, ((index // 8) % 8) * 8 + 4)
            self.clicked.add(point)
            action = GameAction.ACTION6
            action.set_data({"x": point[0], "y": point[1]})
            action.reasoning = {"policy": "ontogenesis", "probe": "visual-object"}
            return action

        def choose_action(
            self, frames: list[FrameData], latest_frame: FrameData
        ) -> GameAction:
            if latest_frame.state in (GameState.NOT_PLAYED, GameState.GAME_OVER):
                if (
                    latest_frame.state is GameState.GAME_OVER
                    and self.controller is not None
                ):
                    self.controller.mark_episode_failure()
                self.awaiting_reset = True
                return GameAction.RESET

            if not latest_frame.frame:
                raise ValueError("ARC harness supplied no observation frame")
            frame = np.asarray(latest_frame.frame[-1], dtype=np.int16)
            available = self._available(latest_frame)
            simple = [
                action
                for action in available
                if action is not GameAction.RESET and action.is_simple()
            ]
            complex_actions = [action for action in available if action.is_complex()]
            if not simple and complex_actions:
                return self._click_action(frame)
            if not simple:
                raise ValueError("ARC harness exposed no playable action")

            names = [action.name for action in simple]
            if self.controller is None:
                self.controller = OntogenesisController(names)
                self.completed_level = latest_frame.levels_completed
            elif latest_frame.levels_completed != self.completed_level:
                inherited = self.controller
                inherited.mark_level_success()
                self.controller = OntogenesisController(names, inherited=inherited)
                self.completed_level = latest_frame.levels_completed
                self.clicked.clear()
                self.clicks.clear()
            elif self.awaiting_reset:
                self.controller.begin_episode()
            self.awaiting_reset = False

            chosen_name = self.controller.choose(frame)
            by_name = {action.name: action for action in simple}
            action = by_name.get(chosen_name, simple[0])
            action.reasoning = {"policy": "ontogenesis", "model": "frame-only"}
            return action
