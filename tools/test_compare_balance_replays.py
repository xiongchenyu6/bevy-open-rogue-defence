#!/usr/bin/env python3
"""Exercise replay comparison through the real CLI using isolated receipts."""

import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class ReplayComparisonTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.records = [
            {"type": "runner_metadata", "binary_sha256": "a" * 64},
            {
                "type": "metadata",
                "chapter": 0,
                "training": 0,
                "policy": "passive",
                "seed": 1,
                "starter": 2,
                "forward": False,
                "frame_cap": 1800,
                "fixed_step_secs": 1 / 60,
                "executor": "single",
            },
            {
                "type": "pressure",
                "movement_growth": 0.03,
                "attack_growth": 0.05,
                "health_growth": 0.1,
                "movement_cap": 1.8,
                "attack_cap": 2.2,
                "health_cap": 3.3,
            },
            {
                "type": "checkpoint",
                "frame": 32,
                "finished": False,
                "kills": 0,
                "skills": [1, 0],
            },
            {
                "type": "checkpoint",
                "frame": 1800,
                "finished": True,
                "kills": 20,
                "skills": [2, 0],
            },
        ]

        for checkpoint in self.records[3:]:
            checkpoint.update(
                actual_chapter=0,
                permanent_upgrades=[0] * 6,
                actual_run_seed=2,
                actual_hero_id=0,
                elapsed=checkpoint["frame"] / 60,
                wave=1,
                level=1,
                hero_hp=800.0,
                max_hero_hp=800.0,
                deaths=0,
                gate_hp=1000.0,
                attacks=10,
                damage_skill_casts=2,
                victory=False,
            )

    def tearDown(self):
        self.temp.cleanup()

    def run_compare(self, left, right, corrupt_line=None):
        paths = [self.root / "left.jsonl", self.root / "right.jsonl"]
        for path, rows in zip(paths, [left, right]):
            path.write_text(
                "Real diagnostic text is allowed before JSON records\n"
                + "\n".join(json.dumps(row) for row in rows)
                + "\n"
            )
        if corrupt_line is not None:
            with paths[1].open("a") as handle:
                handle.write(corrupt_line + "\n")
        result = subprocess.run(
            [
                sys.executable,
                str(Path(__file__).with_name("compare_balance_replays.py")),
                *map(str, paths),
            ],
            capture_output=True,
            text=True,
        )
        return result.returncode, json.loads(result.stdout)

    def test_complete_and_truncated_equal_receipts_are_distinguished(self):
        status, report = self.run_compare(self.records, self.records)
        self.assertEqual(status, 0)
        self.assertTrue(report["complete_replay"])
        self.assertEqual(
            report["comparison_scope"],
            "recorded checkpoints, not all internal simulation state",
        )
        truncated = copy.deepcopy(self.records)
        truncated[-1]["finished"] = False
        status, report = self.run_compare(truncated, truncated)
        self.assertEqual(status, 0)
        self.assertTrue(report["prefix_equal"])
        self.assertFalse(report["complete_replay"])
        self.assertEqual(report["status"], "equal_truncated_prefix")

    def test_first_nested_numeric_difference_reports_field_and_frame(self):
        changed = copy.deepcopy(self.records)
        changed[-1]["skills"][1] = 3
        status, report = self.run_compare(self.records, changed)
        self.assertEqual(status, 1)
        self.assertEqual(report["first_divergence"]["frame"], 1800)
        self.assertEqual(report["first_divergence"]["field"], "checkpoint[1].skills[1]")

    def test_missing_checkpoint_and_unordered_frames_do_not_pass(self):
        status, report = self.run_compare(self.records, self.records[:-1])
        self.assertEqual(status, 1)
        self.assertEqual(report["first_divergence"]["field"], "checkpoint_count")
        self.assertEqual(report["first_divergence"]["frame"], 1800)
        changed = copy.deepcopy(self.records)
        changed[-1]["frame"] = 31
        status, report = self.run_compare(self.records, changed)
        self.assertEqual(status, 2)
        self.assertIn("unordered frame", report["error"])

    def test_binary_identity_and_required_metadata_are_enforced(self):
        changed = copy.deepcopy(self.records)
        changed[0]["binary_sha256"] = "b" * 64
        status, report = self.run_compare(self.records, changed)
        self.assertEqual(status, 1)
        self.assertEqual(
            report["first_divergence"]["field"], "runner_metadata.binary_sha256"
        )
        for index, field in [(0, "binary_sha256"), (1, "executor")]:
            changed = copy.deepcopy(self.records)
            del changed[index][field]
            status, report = self.run_compare(self.records, changed)
            self.assertEqual(status, 2)
            self.assertIn(field, report["error"])

    def test_pressure_identity_and_checkpoint_field_presence_are_strict(self):
        changed = copy.deepcopy(self.records)
        changed[2]["health_growth"] = 0.12
        status, report = self.run_compare(self.records, changed)
        self.assertEqual(status, 1)
        self.assertEqual(report["first_divergence"]["field"], "pressure.health_growth")
        changed = copy.deepcopy(self.records)
        del changed[-1]["kills"]
        status, report = self.run_compare(self.records, changed)
        self.assertEqual(status, 2)
        self.assertIn("checkpoint[1].kills", report["error"])

    def test_corrupt_json_and_nonfinite_numbers_are_not_silently_dropped(self):
        for line in [
            '{"type":"checkpoint",',
            '[{"type":',
            '{"value":NaN}',
            '{"value":Infinity}',
            '{"value":-Infinity}',
            '{"value":1e999}',
        ]:
            with self.subTest(line=line):
                status, report = self.run_compare(
                    self.records, self.records, corrupt_line=line
                )
                self.assertEqual(status, 2)
                self.assertFalse(report["complete_replay"])
                self.assertIn("error", report)
                self.assertEqual(
                    report["comparison_scope"],
                    "recorded checkpoints, not all internal simulation state",
                )

    def test_minimal_or_missing_production_state_is_invalid_even_if_equal(self):
        minimal = copy.deepcopy(self.records)
        minimal[3] = {"type": "checkpoint", "frame": 32, "finished": False}
        status, report = self.run_compare(minimal, minimal)
        self.assertEqual(status, 2)
        self.assertIn("actual_chapter", report["error"])
        for field in [
            "permanent_upgrades",
            "max_hero_hp",
            "damage_skill_casts",
            "victory",
        ]:
            changed = copy.deepcopy(self.records)
            del changed[-1][field]
            status, report = self.run_compare(changed, changed)
            self.assertEqual(status, 2)
            self.assertIn(field, report["error"])


if __name__ == "__main__":
    unittest.main()
