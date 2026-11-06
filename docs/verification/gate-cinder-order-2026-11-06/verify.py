"""Validate archived bytes and actual red/green receipts, not replay determinism."""
from pathlib import Path
import hashlib
import json

root = Path(__file__).resolve().parent
manifest = json.loads((root / "manifest.json").read_text())
assert set(manifest) == {str(p.relative_to(root)) for p in root.rglob("*") if p.is_file() and p.name != "manifest.json"}
for name, row in manifest.items():
    data = (root / name).read_bytes()
    assert len(data) == row["bytes"] and hashlib.sha256(data).hexdigest() == row["sha256"], name
assert json.loads((root / "validation-inputs-before.json").read_text()) == json.loads((root / "validation-inputs-after.json").read_text())
processes = json.loads((root / "validation-processes.json").read_text())
assert [p["stage"] for p in processes] == ["format", "tests", "clippy", "sim-build"]
assert all(p["exit"] == 0 for p in processes)
assert "FAILED. 0 passed; 1 failed" in (root / "red-release.log").read_text()
assert "left: 1086523963" in (root / "red-release.log").read_text()
assert "right: 1086523965" in (root / "red-release.log").read_text()
assert "ok. 115 passed; 0 failed" in (root / "green2.log").read_text()
assert "FAILED. 0 passed; 2 failed" in (root / "cinder-red.log").read_text()
final = (root / "tests.log").read_text()
assert "ok. 737 passed; 0 failed; 1 ignored" in final
rows = [line for line in final.splitlines() if "CINDER_TIED_ORDER reverse=" in line]
assert len(rows) == 4
assert all("hp=[9976.3, 14958.525] rng=16263334312487546840" in line for line in rows)
state = (root / "validation-unit-state.txt").read_text()
assert "ActiveState=inactive" in state and "Result=success" in state and "ExecMainStatus=0" in state
print("Archived red/green evidence PASS; full replay equivalence is unproven")
