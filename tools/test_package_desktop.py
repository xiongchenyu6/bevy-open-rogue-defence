#!/usr/bin/env python3
"""Packaging integration checks use isolated fixtures, never real preferences."""

import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


class PackageDesktopTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / "tools").mkdir()
        shutil.copy2(
            Path(__file__).with_name("package_desktop.py"),
            self.root / "tools/package_desktop.py",
        )
        (self.root / "assets/models").mkdir(parents=True)
        (self.root / "assets/models/model with space.glb").write_bytes(
            b"actual fixture asset bytes"
        )
        (self.root / "assets/licenses").mkdir()
        (self.root / "assets/licenses/SOURCE.md").write_text(
            "Retain the original source and license documentation.\n"
        )
        (self.root / "docs").mkdir()
        (self.root / "docs/DESKTOP_PLAYER_README.md").write_text(
            "Launch Rogue Commander and defend the gate.\n"
        )
        (self.root / "Cargo.toml").write_text(
            '[package]\nname="own"\nversion="0.1.0"\n'
        )
        self.binary = self.root / "fake-elf"
        self.binary.write_bytes(b"\x7fELFdebug packaging fixture")
        self.binary.chmod(0o755)
        packages = []
        for name in ["own", "normal", "build", "dev"]:
            directory = self.root if name == "own" else self.root / "registry" / name
            directory.mkdir(parents=True, exist_ok=True)
            (directory / "Cargo.toml").touch()
            packages.append(
                {
                    "id": name,
                    "name": name,
                    "version": "0.1.0",
                    "license": "MIT",
                    "repository": "https://example.invalid/" + name,
                    "manifest_path": str(directory / "Cargo.toml"),
                }
            )
            if name != "build":
                (directory / "LICENSE-MIT").write_text(
                    name + " original license evidence\n"
                )

        def edge(name, kind):
            return {"pkg": name, "dep_kinds": [{"kind": kind, "target": None}]}

        self.metadata = self.root / "metadata.json"
        self.metadata.write_text(
            json.dumps(
                {
                    "packages": packages,
                    "resolve": {
                        "root": "own",
                        "nodes": [
                            {
                                "id": "own",
                                "deps": [edge("normal", None), edge("dev", "dev")],
                            },
                            {"id": "normal", "deps": [edge("build", "build")]},
                            {"id": "build", "deps": []},
                            {"id": "dev", "deps": []},
                        ],
                    },
                }
            )
        )
        self.output = self.root / "package"

    def tearDown(self):
        self.temp.cleanup()

    def run_tool(self, *extra, env=None):
        return subprocess.run(
            [
                sys.executable,
                str(self.root / "tools/package_desktop.py"),
                "--binary",
                str(self.binary),
                "--metadata",
                str(self.metadata),
                "--output",
                str(self.output),
                "--profile",
                "debug",
                "--target",
                "x86_64-unknown-linux-gnu",
                *extra,
            ],
            capture_output=True,
            text=True,
            env=env,
        )

    def test_complete_assets_missing_evidence_and_manifest(self):
        result = self.run_tool("--allow-missing-licenses")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.output / "assets/models/model with space.glb").read_bytes(),
            (self.root / "assets/models/model with space.glb").read_bytes(),
        )
        self.assertTrue((self.output / "assets/licenses/SOURCE.md").is_file())
        index = json.loads((self.output / "third-party-index.json").read_text())
        self.assertEqual({p["name"] for p in index["packages"]}, {"normal", "build"})
        self.assertEqual(index["missing_license_files"][0]["name"], "build")
        self.assertIn("Not performed", index["legal_review"])
        version = json.loads((self.output / "version.json").read_text())
        self.assertFalse(version["release_package"])
        for row in json.loads((self.output / "package-manifest.json").read_text())[
            "files"
        ]:
            data = (self.output / row["file"]).read_bytes()
            self.assertEqual(hashlib.sha256(data).hexdigest(), row["sha256"])
        repeated = self.run_tool("--allow-missing-licenses")
        self.assertEqual(repeated.returncode, 2)
        self.assertIn("already exists", repeated.stderr)
        self.assertTrue((self.output / "rogue_commander").is_file())

    def test_missing_licenses_fail_without_partial_output(self):
        result = self.run_tool()
        self.assertEqual(result.returncode, 2)
        self.assertIn("Missing license files", result.stderr)
        self.assertIn("build", result.stderr)
        self.assertFalse(self.output.exists())
        self.assertFalse(list(self.root.glob(".package-*")))
        release = self.run_tool("--profile", "release", "--allow-missing-licenses")
        self.assertEqual(release.returncode, 2)
        self.assertIn("only for explicit debug", release.stderr)

    def test_pinned_license_override_and_hash_verification(self):
        source = self.root / "licenses/rust/build-0.1.0/LICENSE-MIT"
        source.parent.mkdir(parents=True)
        source.write_text("Original pinned license text\n")
        override = {
            "packages": [
                {
                    "name": "build",
                    "version": "0.1.0",
                    "spdx": "MIT",
                    "status": "downloaded",
                    "commit": "abc123",
                    "files": [
                        {
                            "path": source.relative_to(self.root).as_posix(),
                            "source_url": "https://example.invalid/abc123/LICENSE-MIT",
                            "sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
                        }
                    ],
                }
            ]
        }
        (self.root / "licenses/rust-overrides.json").write_text(json.dumps(override))
        result = self.run_tool()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.output / "third-party/build-0.1.0/LICENSE-MIT").read_text(),
            source.read_text(),
        )
        shutil.rmtree(self.output)
        source.write_text("tampered")
        result = self.run_tool()
        self.assertEqual(result.returncode, 2)
        self.assertIn("SHA256 mismatch", result.stderr)
        self.assertFalse(self.output.exists())

    def test_blocked_override_remains_missing_even_with_registry_notice(self):
        overrides = {
            "packages": [
                {
                    "name": "normal",
                    "version": "0.1.0",
                    "spdx": "MIT",
                    "status": "blocked",
                    "files": [],
                    "blocked_reason": "Original license text not retained",
                }
            ]
        }
        (self.root / "licenses").mkdir()
        (self.root / "licenses/rust-overrides.json").write_text(json.dumps(overrides))
        failed = self.run_tool()
        self.assertEqual(failed.returncode, 2)
        self.assertIn("normal", failed.stderr)
        self.assertIn("build", failed.stderr)
        self.assertFalse(self.output.exists())
        allowed = self.run_tool("--allow-missing-licenses")
        self.assertEqual(allowed.returncode, 0, allowed.stderr)
        index = json.loads((self.output / "third-party-index.json").read_text())
        self.assertEqual(
            {row["name"] for row in index["missing_license_files"]}, {"normal", "build"}
        )
        self.assertEqual(
            next(
                row for row in index["missing_license_files"] if row["name"] == "normal"
            )["status"],
            "blocked",
        )

    def test_release_rejects_absolute_nix_linkage(self):
        fake = self.root / "bin/readelf"
        fake.parent.mkdir()
        fake.write_text(
            '#!/usr/bin/env python3\nprint("[Requesting program interpreter: /nix/store/example/ld-linux.so]")\n'
        )
        fake.chmod(0o755)
        env = dict(os.environ, PATH=str(fake.parent) + os.pathsep + os.environ["PATH"])
        result = self.run_tool("--profile", "release", env=env)
        self.assertEqual(result.returncode, 2)
        self.assertIn("absolute Nix dependencies", result.stderr)
        self.assertFalse(self.output.exists())


if __name__ == "__main__":
    unittest.main()
