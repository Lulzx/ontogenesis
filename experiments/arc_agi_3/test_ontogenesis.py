import unittest

import numpy as np
from ontogenesis import (
    AcquiredTranslation,
    LatentSignature,
    OntogenesisController,
    StatusEdgeEffect,
    VisualTarget,
    infer_translation,
)


class OntogenesisTests(unittest.TestCase):
    def test_infers_opaque_translation_from_pixels(self) -> None:
        before = np.full((64, 64), 3, dtype=np.int16)
        before[30:35, 20:25] = 9
        after = np.full((64, 64), 3, dtype=np.int16)
        after[25:30, 20:25] = 9
        self.assertEqual(infer_translation(before, after), (0, -5, 25))

    def test_acquires_action_law_and_mover(self) -> None:
        before = np.full((64, 64), 3, dtype=np.int16)
        before[30:35, 20:25] = 9
        after = np.full((64, 64), 3, dtype=np.int16)
        after[30:35, 25:30] = 9
        controller = OntogenesisController(["opaque"])
        controller.previous = before
        controller.last_action = "opaque"
        controller.probes.clear()
        controller._learn_transition(before, after, "opaque")
        self.assertEqual(
            (
                controller.translations["opaque"].dx,
                controller.translations["opaque"].dy,
            ),
            (5, 0),
        )
        self.assertEqual(controller.mover_anchor, (25, 30))
        self.assertEqual(controller.floor_color, 3)

    def test_latent_rotation_is_executable(self) -> None:
        signature = LatentSignature(
            9,
            (
                True,
                True,
                True,
                False,
                False,
                True,
                True,
                False,
                True,
            ),
        )
        goal = signature
        for _ in range(3):
            goal = goal.rotate_clockwise()
        self.assertEqual(OntogenesisController._rotation_distance(signature, goal), 3)

    def test_transfers_frozen_action_laws_without_reprobing(self) -> None:
        source = OntogenesisController(["up", "down"])
        source.translations = {
            "up": AcquiredTranslation("up", 0, -5, 25),
            "down": AcquiredTranslation("down", 0, 5, 25),
        }
        source.stride = 5
        source.mover_colors = frozenset({9, 12})
        source.floor_color = 3
        transferred = OntogenesisController(["up", "down"], inherited=source)
        self.assertEqual(list(transferred.probes), [])
        self.assertEqual(transferred.translations, source.translations)
        self.assertEqual(transferred.stride, 5)

    def test_reads_visible_action_budget(self) -> None:
        frame = np.full((64, 64), 5, dtype=np.int16)
        frame[61:63, 13:42] = 11
        self.assertEqual(OntogenesisController._action_budget(frame), 29)

    def test_reset_drops_pending_motor_state_but_keeps_acquired_laws(self) -> None:
        controller = OntogenesisController(["up"])
        controller.translations["up"] = AcquiredTranslation("up", 0, -5, 25)
        controller.previous = np.zeros((64, 64), dtype=np.int16)
        controller.last_action = "up"
        controller.mover_anchor = (20, 20)
        controller.initial_mover_anchor = (10, 10)
        controller.mover_colors = frozenset({2})
        controller.initial_mover_colors = frozenset({9})
        controller.resources_collected = 2
        controller.visited_targets.add((30, 30))
        controller.plan.extend(["up"])
        controller.begin_episode()
        self.assertIsNone(controller.previous)
        self.assertIsNone(controller.last_action)
        self.assertEqual(controller.mover_anchor, (10, 10))
        self.assertEqual(controller.mover_colors, frozenset({9}))
        self.assertEqual(controller.resources_collected, 0)
        self.assertEqual(controller.visited_targets, set())
        self.assertEqual(list(controller.plan), [])
        self.assertIn("up", controller.translations)

    def test_fallback_explores_each_action_at_a_state(self) -> None:
        controller = OntogenesisController(["a", "b"])
        controller.probes.clear()
        controller.mover_anchor = (10, 10)
        first = controller._least_used_action()
        controller.edge_attempts[((10, 10), first)] += 1
        controller.action_attempts[first] += 1
        self.assertNotEqual(controller._least_used_action(), first)

    def test_successful_interaction_becomes_inherited_goal_prototype(self) -> None:
        controller = OntogenesisController(["move"])
        target = VisualTarget((20, 20), "state_modifier", frozenset({2, 7}), 9)
        controller.active_target = target
        controller.mark_level_success()
        inherited = OntogenesisController(["move"], inherited=controller)
        self.assertIn(
            (target.colors, target.size), inherited.successful_target_features
        )

    def test_extracts_single_color_goal_glyph(self) -> None:
        frame = np.full((64, 64), 3, dtype=np.int16)
        frame[20:23, 20] = 9
        frame[22, 20:23] = 9
        target = VisualTarget((20, 20), "goal_analogue", frozenset({9}), 5)
        controller = OntogenesisController(["move"])
        controller.stride = 5
        signature = controller._goal_signature(frame, target)
        self.assertIsNotNone(signature)
        self.assertEqual(signature.color, 9)
        self.assertEqual(sum(signature.pattern), 5)

    def test_sprite_geometry_does_not_change_navigation_lattice(self) -> None:
        controller = OntogenesisController(["move"])
        controller.stride = 5
        controller.grid_phase = (4, 0)
        self.assertEqual(controller._snap_anchor((13, 39)), (14, 40))
        self.assertEqual(controller._snap_anchor((16, 42)), (14, 40))

    def test_learned_rotation_edge_generalizes_to_current_signature(self) -> None:
        before = LatentSignature(
            9, (True, True, False, False, True, False, True, False, False)
        )
        after = before.rotate_clockwise()
        current = after
        effect = StatusEdgeEffect((10, 10), "move", (15, 10), before, after)
        self.assertEqual(
            OntogenesisController._apply_status_effect(effect, current),
            current.rotate_clockwise(),
        )

    def test_status_planner_respects_failed_edge_quotient(self) -> None:
        controller = OntogenesisController(["move"])
        before = LatentSignature(
            9, (True, True, False, False, True, False, True, False, False)
        )
        after = before.rotate_clockwise()
        effect = StatusEdgeEffect((10, 10), "move", (15, 10), before, after)
        controller.status_edge_effects[(effect.origin, effect.action)] = effect
        controller.blocked_edges.add((effect.origin, effect.action))
        controller.mover_anchor = effect.origin
        controller.goal_signature_model = after
        frame = np.full((64, 64), 3, dtype=np.int16)
        frame[53:63, 1:11] = 3
        pattern = np.asarray(before.pattern, dtype=bool).reshape(3, 3)
        frame[55:58, 3:6][pattern] = before.color
        self.assertFalse(controller._plan_status_option(frame))

    def test_product_path_avoids_latent_state_regression(self) -> None:
        controller = OntogenesisController(["right", "left", "down", "up"])
        controller.translations = {
            "right": AcquiredTranslation("right", 5, 0, 25),
            "left": AcquiredTranslation("left", -5, 0, 25),
            "down": AcquiredTranslation("down", 0, 5, 25),
            "up": AcquiredTranslation("up", 0, -5, 25),
        }
        controller.stride = 5
        controller.floor_color = 3
        controller.mover_anchor = (10, 10)
        status = LatentSignature(
            9, (True, True, False, False, True, False, True, False, False)
        )
        regressed = LatentSignature(12, status.pattern)
        controller.status_edge_effects[((10, 10), "right")] = StatusEdgeEffect(
            (10, 10), "right", (15, 10), status, regressed
        )
        frame = np.full((64, 64), 3, dtype=np.int16)
        route = controller._shortest_product_path(frame, (20, 10), status, status)
        self.assertNotEqual(route[0], "right")
        self.assertEqual(route, ["down", "right", "right", "up"])

    def test_product_path_can_enter_verified_modifier_sprite(self) -> None:
        controller = OntogenesisController(["right"])
        controller.translations = {
            "right": AcquiredTranslation("right", 5, 0, 25),
        }
        controller.stride = 5
        controller.floor_color = 3
        controller.mover_anchor = (10, 10)
        before = LatentSignature(
            12, (True, True, False, False, True, False, True, False, False)
        )
        after = LatentSignature(9, before.pattern)
        controller.status_edge_effects[((10, 10), "right")] = StatusEdgeEffect(
            (10, 10), "right", (15, 10), before, after
        )
        frame = np.full((64, 64), 3, dtype=np.int16)
        frame[10:15, 15:20] = 7
        route = controller._shortest_product_path(frame, (20, 10), before, after)
        self.assertEqual(route, ["right", "right"])

    def test_expected_product_transport_preserves_remaining_plan(self) -> None:
        controller = OntogenesisController(["right"])
        controller.translations["right"] = AcquiredTranslation(
            "right", 5, 0, 25
        )
        controller.last_origin = (10, 10)
        controller.last_action = "right"
        controller.mover_anchor = (30, 30)
        controller.transition_edges[((10, 10), "right")] = (30, 30)
        controller.active_target = VisualTarget(
            (40, 30), "goal_analogue", frozenset({9}), 5
        )
        controller.last_was_planned = True
        controller.plan_is_product = True
        controller.plan.extend(["right", "right"])
        controller._validate_transition(np.zeros((64, 64), dtype=np.int16))
        self.assertEqual(list(controller.plan), ["right", "right"])
        self.assertTrue(controller.last_was_expected_product_transport)

    def test_visual_transition_schema_generalizes_repeated_portal_tile(self) -> None:
        controller = OntogenesisController(["right", "left"])
        controller.translations = {
            "right": AcquiredTranslation("right", 5, 0, 25),
            "left": AcquiredTranslation("left", -5, 0, 25),
        }
        controller.stride = 5
        controller.floor_color = 3
        controller.mover_anchor = (10, 10)
        frame = np.full((64, 64), 3, dtype=np.int16)
        frame[10:15, 15:20] = 7
        frame[20:25, 25:30] = 7
        key = controller._tile_key(frame, (15, 10))
        self.assertIsNotNone(key)
        controller._learn_tile_transition(key, (15, 10), (25, 20))
        controller._learn_tile_transition(key, (15, 20), (25, 20))
        route = controller._shortest_path(frame, (30, 20))
        self.assertEqual(route, ["right", "right"])

    def test_visual_transition_schema_acquires_relative_conveyor(self) -> None:
        controller = OntogenesisController(["move"])
        key = (7, 7, 7, 7)
        controller._learn_tile_transition(key, (10, 10), (15, 10))
        controller._learn_tile_transition(key, (20, 20), (25, 20))
        schema = controller.transition_tile_effects[key]
        self.assertEqual(schema.mode, "relative")
        self.assertEqual(schema.resolve((30, 30)), (35, 30))

    def test_entry_cell_transition_generalizes_across_approaches(self) -> None:
        controller = OntogenesisController(["right", "down"])
        controller.translations = {
            "right": AcquiredTranslation("right", 5, 0, 25),
            "down": AcquiredTranslation("down", 0, 5, 25),
        }
        controller.stride = 5
        controller.floor_color = 3
        controller.mover_anchor = (10, 10)
        controller.transition_entry_effects[(15, 10)] = (30, 30)
        frame = np.full((64, 64), 3, dtype=np.int16)
        self.assertEqual(controller._shortest_path(frame, (30, 35)), ["right", "down"])

    def test_status_change_takes_priority_over_simultaneous_budget_gain(self) -> None:
        before = LatentSignature(
            9, (True, True, False, True, False, False, True, True, True)
        )
        after = before.rotate_clockwise()
        self.assertEqual(
            OntogenesisController._status_effect_kind(before, after), "rotation"
        )

    def test_status_change_invalidates_stale_target_order(self) -> None:
        controller = OntogenesisController(["move"])
        controller.last_origin = (20, 20)
        controller.last_action = "move"
        controller.mover_anchor = (25, 20)
        controller.targets.append(
            VisualTarget((30, 20), "state_modifier", frozenset({7}), 25)
        )
        before = np.full((64, 64), 5, dtype=np.int16)
        after = before.copy()
        before[53:63, 1:11] = 3
        after[53:63, 1:11] = 3
        before[55:57, 3:5] = 9
        before[55:57, 5:7] = 9
        after[55:57, 5:7] = 9
        after[57:59, 5:7] = 9
        controller._learn_status_edge(before, after)
        self.assertEqual(list(controller.targets), [])
        self.assertIn(((20, 20), "move"), controller.status_edge_effects)

    def test_budget_spend_is_not_mistaken_for_target_interaction(self) -> None:
        controller = OntogenesisController(["move"])
        controller.stride = 5
        controller.mover_anchor = (20, 20)
        controller.active_target = VisualTarget(
            (25, 20), "state_modifier", frozenset({7}), 25
        )
        controller.active_before_budget = 20
        controller.active_before = None
        controller.active_target_pixels = 25
        controller.plan.clear()
        before = np.full((64, 64), 3, dtype=np.int16)
        after = before.copy()
        for x in range(12, 55):
            before[61:63, x] = 6 + x % 2
            after[61:63, x] = 6 + x % 2
        before[61:63, 12:32] = 11
        after[61:63, 12:31] = 11
        controller.previous = before
        self.assertFalse(controller._interaction_observed(after))

    def test_terminal_failure_blocks_only_the_last_edge(self) -> None:
        controller = OntogenesisController(["move"])
        controller.last_origin = (20, 20)
        controller.last_action = "move"
        controller.last_was_planned = True
        controller.active_target = VisualTarget(
            (25, 20), "state_modifier", frozenset({7}), 8
        )
        detour = (frozenset({7}), 8)
        controller.matched_detours_this_episode.add(detour)
        controller.plan.extend(["move", "move"])
        controller.mark_episode_failure()
        self.assertIn(((20, 20), "move"), controller.blocked_edges)
        self.assertIn(detour, controller.failed_matched_detours)
        self.assertEqual(list(controller.plan), [])

    def test_unscoped_terminal_failure_does_not_poison_final_edge(self) -> None:
        controller = OntogenesisController(["move"])
        controller.last_origin = (20, 20)
        controller.last_action = "move"
        controller.mark_episode_failure()
        self.assertNotIn(((20, 20), "move"), controller.blocked_edges)

    def test_product_timeout_does_not_poison_intermediate_edge(self) -> None:
        controller = OntogenesisController(["move"])
        controller.last_origin = (20, 20)
        controller.last_action = "move"
        controller.last_was_planned = True
        controller.plan_is_product = True
        controller.active_target = VisualTarget(
            (50, 50), "goal_analogue", frozenset({9}), 5
        )
        controller.mark_episode_failure()
        self.assertNotIn(((20, 20), "move"), controller.blocked_edges)

    def test_remote_scene_change_excludes_local_motion_and_hud(self) -> None:
        controller = OntogenesisController(["move"])
        controller.stride = 5
        controller.last_origin = (20, 20)
        target = VisualTarget((25, 20), "state_modifier", frozenset({7}), 25)
        before = np.full((64, 64), 3, dtype=np.int16)
        before[20:25, 20:25] = 9
        before[20:25, 25:30] = 7
        after = np.full((64, 64), 3, dtype=np.int16)
        after[20:25, 25:30] = 9
        after[61:63, 12:30] = 11
        self.assertFalse(controller._remote_scene_changed(before, after, target))
        before[35:40, 40:45] = 8
        self.assertTrue(controller._remote_scene_changed(before, after, target))


if __name__ == "__main__":
    unittest.main()
