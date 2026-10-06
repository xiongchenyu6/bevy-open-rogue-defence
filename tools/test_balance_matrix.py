#!/usr/bin/env python3
"""Real subprocess tests of matrix orchestration; fake binaries do not simulate balance."""

import csv
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class BalanceMatrixTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "tools").mkdir()
        for name in ["run_balance_matrix.py", "analyze_balance.py"]:
            shutil.copy2(Path(__file__).with_name(name), self.root / "tools" / name)
        (self.root / "src/game").mkdir(parents=True)
        (self.root / "src/game/campaign_progress.rs").write_text(
            "pub const MAX_UPGRADE_LEVEL: u8 = 10;\n"
        )
        self.binary = self.root / "fake-simulator"
        self.binary.write_text("""#!/usr/bin/env python3
import json,os,sys,time
if os.getenv('FAKE_SLEEP'): time.sleep(float(os.environ['FAKE_SLEEP']))
chapter=int(os.environ['ROGUE_SIM_CHAPTER']);training=int(os.environ['ROGUE_SIM_TRAINING'])
print(json.dumps({'type':'metadata','chapter':chapter,'training':training,'policy':sys.argv[1],'seed':int(sys.argv[2],16),'starter':int(sys.argv[3]),'forward':sys.argv[4]=='forward','frame_cap':int(sys.argv[5]),'fixed_step_secs':1/60,'preferences':{key:os.environ[key] for key in ['XDG_CONFIG_HOME','ROGUE_CAMPAIGN_SAVE','ROGUE_DETAIL_SAVE']},'pressure_args':sys.argv[6:]}),flush=True)
print(json.dumps({'type':'checkpoint','frame':int(sys.argv[5]),'elapsed':1,'actual_run_seed':1,'actual_hero_id':0,'actual_chapter':chapter+int(os.getenv('FAKE_MISMATCH','0')),'permanent_upgrades':[training]*6,'gate_hp':1000,'hero_hp':800,'damage_skill_casts':0,'finished':False,'victory':True}),flush=True)
""")
        self.binary.chmod(0o755)

    def tearDown(self):
        self.temp.cleanup()

    def run_matrix(self, *extra, env=None):
        return subprocess.run(
            [
                sys.executable,
                str(self.root / "tools/run_balance_matrix.py"),
                "--binary",
                str(self.binary),
                "--batch",
                "test",
                "--frames",
                "10",
                "--seeds",
                "1",
                "--starters",
                "2",
                "--positions",
                "behind",
                "--policies",
                "passive",
                *extra,
            ],
            capture_output=True,
            text=True,
            env=env,
        )

    def summary(self):
        return json.loads(
            (self.root / "target/verification/test/summary.json").read_text()
        )

    def test_parallel_dimensions_isolated_environment_and_censored_analysis(self):
        result = self.run_matrix(
            "--chapters",
            "0,5",
            "--training",
            "0,10",
            "--policies",
            "passive,active",
            "--jobs",
            "2",
            "--require-telemetry",
            "--pressure",
            "0,0,1,1,.1,3.3",
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = self.summary()
        self.assertEqual(len(summary["cases"]), 8)
        configs = set()
        for case in summary["cases"]:
            self.assertTrue(case["valid"])
            self.assertTrue(case["dimension_valid"])
            self.assertTrue(case["censored"])
            self.assertEqual(case["outcome"], "frame_cap")
            self.assertIn(f"c{case['chapter']}-t{case['training']}", case["name"])
            rows = [
                json.loads(line) for line in Path(case["log"]).read_text().splitlines()
            ]
            metadata = next(row for row in rows if row["type"] == "metadata")
            self.assertEqual(metadata["chapter"], case["chapter"])
            self.assertEqual(metadata["training"], case["training"])
            configs.add(metadata["preferences"]["XDG_CONFIG_HOME"])
            self.assertEqual(
                metadata["pressure_args"], ["0", "0", "1", "1", ".1", "3.3"]
            )
        self.assertEqual(len(configs), 8)
        analyzed = self.root / "analysis"
        result = subprocess.run(
            [
                sys.executable,
                str(self.root / "tools/analyze_balance.py"),
                str(self.root / "target/verification/test"),
                "--output",
                str(analyzed),
            ],
            capture_output=True,
            text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        with (analyzed / "aggregates.csv").open() as handle:
            aggregates = list(csv.DictReader(handle))
        self.assertEqual(len(aggregates), 8)
        self.assertTrue(
            all(row["wins"] == "0" and row["censored"] == "1" for row in aggregates)
        )
        self.assertEqual(
            {(row["chapter"], row["training"]) for row in aggregates},
            {("0", "0"), ("0", "10"), ("5", "0"), ("5", "10")},
        )

    def test_invalid_dimensions_and_jobs_do_not_start_binary(self):
        for option, value in [
            ("--chapters", "6"),
            ("--training", "11"),
            ("--training", "-1"),
            ("--jobs", "0"),
            ("--jobs", "9"),
        ]:
            result = self.run_matrix(option, value)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertFalse((self.root / "target").exists())

    def test_metadata_mismatch_is_persisted_as_invalid(self):
        result = self.run_matrix(env=dict(os.environ, FAKE_MISMATCH="1"))
        self.assertNotEqual(result.returncode, 0)
        case = self.summary()["cases"][0]
        self.assertFalse(case["dimension_valid"])
        self.assertFalse(case["valid"])
        self.assertEqual(case["outcome"], "invalid_receipt")
        self.assertTrue(Path(case["log"]).exists())

    def test_timeout_retains_case_and_logs(self):
        result = self.run_matrix(
            "--case-timeout", ".1", env=dict(os.environ, FAKE_SLEEP="2")
        )
        self.assertNotEqual(result.returncode, 0)
        case = self.summary()["cases"][0]
        self.assertTrue(case["timed_out"])
        self.assertFalse(case["valid"])
        self.assertIn("wall timeout", Path(case["stderr"]).read_text())


if __name__ == "__main__":
    unittest.main()
