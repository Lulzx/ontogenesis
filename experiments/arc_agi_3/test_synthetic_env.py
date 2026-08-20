import unittest

import numpy as np
from synthetic_env import (
    SyntheticEnvironment,
    SyntheticSpec,
    load_manifest,
    run_controller,
)


class SyntheticEnvironmentTests(unittest.TestCase):
    def test_seed_fixes_frame_and_opaque_action_binding(self) -> None:
        spec = SyntheticSpec("deterministic", 42, ("rotate", "transport"), 80)
        first = SyntheticEnvironment(spec)
        second = SyntheticEnvironment(spec)
        np.testing.assert_array_equal(first.frame(), second.frame())
        self.assertEqual(first.action_deltas, second.action_deltas)
        self.assertEqual(first.entities, second.entities)

    def test_action_binding_is_not_fixed_across_seeds(self) -> None:
        first = SyntheticEnvironment(SyntheticSpec("first", 1, (), 40))
        second = SyntheticEnvironment(SyntheticSpec("second", 2, (), 40))
        self.assertNotEqual(first.action_deltas, second.action_deltas)

    def test_rotation_is_observable_but_not_named_in_action_space(self) -> None:
        env = SyntheticEnvironment(SyntheticSpec("rotation", 7, ("rotate",), 80))
        modifier = next(entity for entity in env.entities if entity.kind == "rotate")
        before = env.status
        env.avatar = modifier.anchor
        env._interact(modifier.anchor)
        self.assertEqual(env.status, before.rotate_clockwise())
        self.assertNotIn("rotate", env.action_names)

    def test_manifest_has_frozen_disjoint_seeds_and_novel_heldout_mechanics(self) -> None:
        manifest = load_manifest()
        curriculum = manifest["curriculum"]
        heldout = manifest["heldout"]
        self.assertTrue({spec.seed for spec in curriculum}.isdisjoint(
            spec.seed for spec in heldout
        ))
        curriculum_mechanics = {m for spec in curriculum for m in spec.mechanics}
        heldout_mechanics = {m for spec in heldout for m in spec.mechanics}
        self.assertTrue({"toggle", "hazard", "conveyor"} <= heldout_mechanics)
        self.assertTrue({"toggle", "hazard", "conveyor"}.isdisjoint(curriculum_mechanics))

    def test_controller_evaluation_is_bounded(self) -> None:
        spec = SyntheticSpec("bounded", 99, (), 20)
        result = run_controller(spec, max_resets=0)
        self.assertLessEqual(result.actions, spec.max_actions)
        self.assertEqual(result.resets, 0)


if __name__ == "__main__":
    unittest.main()
