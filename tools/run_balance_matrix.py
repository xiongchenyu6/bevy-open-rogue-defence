#!/usr/bin/env python3
"""Bounded real-GamePlugin balance matrix; every case has fresh isolated saves."""

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--binary", required=True)
parser.add_argument("--batch", required=True)
parser.add_argument("--frames", type=int, default=90000)
parser.add_argument("--seeds", default="1,2,3,4,5,6,7,8")
parser.add_argument("--policies", default="passive,active")
parser.add_argument("--starters", default="0,1,2")
parser.add_argument("--positions", default="forward,behind")
parser.add_argument(
    "--pressure",
    default="",
    help="movement_growth,attack_growth,movement_cap,attack_cap,health_growth,health_cap (new binary only)",
)
parser.add_argument(
    "--require-telemetry",
    action="store_true",
    help="Reject stale binaries without actual damage-skill cast telemetry",
)
args = parser.parse_args()
binary = Path(args.binary).resolve()
if not binary.is_file() or not os.access(binary, os.X_OK):
    parser.error("--binary must be an executable file")
if args.frames < 1:
    parser.error("--frames must be at least 1")
if Path(args.batch).name != args.batch or args.batch in ("", ".", ".."):
    parser.error("--batch must be one directory name")
seeds = args.seeds.split(",")
try:
    values = [int(seed.removeprefix("0x"), 16) for seed in seeds]
except ValueError:
    parser.error("--seeds must be comma-separated hexadecimal integers")
if any((value < 0 or value >= 2**64 for value in values)):
    parser.error("--seeds must fit unsigned 64-bit integers")
seeds = [format(value, "x") for value in values]
starters = args.starters.split(",")
if not starters or any((starter not in ("0", "1", "2") for starter in starters)):
    parser.error("--starters must contain only 0,1,2")
policies = args.policies.split(",")
if any((policy not in ("passive", "active") for policy in policies)):
    parser.error("--policies must contain only passive,active")
positions = args.positions.split(",")
if any((position not in ("forward", "behind") for position in positions)):
    parser.error("--positions must contain only forward,behind")
pressure = args.pressure.split(",") if args.pressure else []
if pressure:
    import math

    try:
        pressure_values = [float(value) for value in pressure]
    except ValueError:
        parser.error("--pressure must contain six finite floats")
    if len(pressure_values) != 6 or not all(
        (math.isfinite(value) for value in pressure_values)
    ):
        parser.error("--pressure must contain six finite floats")
output = ROOT / "target/verification" / args.batch
output.mkdir(parents=True, exist_ok=True)
digest = hashlib.sha256()
with binary.open("rb") as handle:
    for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
        digest.update(chunk)
binary_sha256 = digest.hexdigest()
manifest = {
    "binary": str(Path(args.binary).resolve()),
    "binary_sha256": binary_sha256,
    "require_telemetry": args.require_telemetry,
    "pressure_override": pressure,
    "frames": args.frames,
    "fixed_step_secs": 1 / 60,
    "cases": [],
    "limitations": [
        "Stationary policies; active greedy card zero is not an optimal strategy.",
        "Frame cap includes menu pause frames; compare actual elapsed time.",
        "Old baseline damage-skill telemetry is unavailable and reported null.",
    ],
}
(output / "summary.json").write_text(json.dumps(manifest, indent=2))
for seed in seeds:
    for starter in starters:
        for position in positions:
            for policy in policies:
                name = f"{policy}-h{starter}-{position}-s{seed}"
                sandbox = output / (name + "-state")
                if sandbox.exists():
                    shutil.rmtree(sandbox)
                sandbox.mkdir()
                env = os.environ.copy()
                env.update(
                    XDG_CONFIG_HOME=str(sandbox / "config"),
                    ROGUE_CAMPAIGN_SAVE=str(sandbox / "campaign.txt"),
                    ROGUE_DETAIL_SAVE=str(sandbox / "detail.txt"),
                )
                log = output / (name + ".jsonl")
                error = output / (name + ".stderr")
                t = time.monotonic()
                print(f"BALANCE_START {name}", flush=True)
                with log.open("w") as out, error.open("w") as err:
                    out.write(
                        json.dumps(
                            {
                                "type": "runner_metadata",
                                "binary_sha256": binary_sha256,
                                "require_telemetry": args.require_telemetry,
                            }
                        )
                        + "\n"
                    )
                    out.flush()
                    result = subprocess.run(
                        [
                            str(Path(args.binary).resolve()),
                            policy,
                            seed,
                            starter,
                            position,
                            str(args.frames),
                        ]
                        + pressure,
                        cwd=ROOT,
                        env=env,
                        stdout=out,
                        stderr=err,
                        timeout=3600,
                    )
                records = []
                for line in log.read_text().splitlines():
                    try:
                        records.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
                checkpoints = [r for r in records if r.get("type") == "checkpoint"]
                case = {
                    "name": name,
                    "binary_sha256": binary_sha256,
                    "returncode": result.returncode,
                    "runtime_secs": round(time.monotonic() - t, 2),
                    "log": str(log),
                    "stderr": str(error),
                    "checkpoints": len(checkpoints),
                    "last": checkpoints[-1] if checkpoints else None,
                }
                last = case["last"]
                case["telemetry_valid"] = not args.require_telemetry or (
                    bool(checkpoints)
                    and all(
                        (r.get("damage_skill_casts") is not None for r in checkpoints)
                    )
                )
                case["valid"] = (
                    result.returncode == 0
                    and bool(checkpoints)
                    and case["telemetry_valid"]
                )
                case["outcome"] = (
                    "win"
                    if last and last.get("victory")
                    else "loss"
                    if last and last.get("finished")
                    else "input_blocked"
                    if last and last.get("paused")
                    else "frame_cap"
                )
                if not case["valid"]:
                    case["outcome"] = "invalid_receipt"
                case["censored"] = (
                    not bool(last and last.get("finished")) or not case["valid"]
                )
                manifest["cases"].append(case)
                (output / "summary.json").write_text(json.dumps(manifest, indent=2))
                print("BALANCE_DONE " + json.dumps(case), flush=True)
                if result.returncode or not checkpoints:
                    raise SystemExit(f"Failed case {name}; inspect durable logs")
                if args.require_telemetry and any(
                    (r.get("damage_skill_casts") is None for r in checkpoints)
                ):
                    raise SystemExit(
                        f"Stale or incompatible binary: {name} lacks required skill-cast telemetry; receipt SHA256={binary_sha256}"
                    )
print(f"BALANCE_MATRIX_COMPLETE {len(manifest['cases'])}", flush=True)
