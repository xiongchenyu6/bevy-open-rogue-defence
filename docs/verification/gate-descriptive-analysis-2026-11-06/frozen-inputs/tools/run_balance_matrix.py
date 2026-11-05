#!/usr/bin/env python3
"""Bounded real-GamePlugin balance matrix; every case has fresh isolated saves."""

import argparse
import hashlib
import json
import itertools
import re
import os
import shutil
import subprocess
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--binary", required=True)
parser.add_argument("--batch", required=True)
parser.add_argument("--frames", type=int, default=90000)
parser.add_argument(
    "--jobs", type=int, default=1, help="Concurrent isolated simulations (1..8)"
)
parser.add_argument(
    "--case-timeout",
    type=float,
    default=3600,
    help="Maximum wall seconds per subprocess",
)
parser.add_argument(
    "--chapters", default="0", help="Comma-separated zero-based chapters 0..5"
)
parser.add_argument(
    "--training",
    default="0",
    help="Comma-separated permanent upgrade levels, applied equally to all six upgrades",
)
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
if not 1 <= args.jobs <= 8:
    parser.error("--jobs must be in 1..8")
if not 0 < args.case_timeout < float("inf"):
    parser.error("--case-timeout must be finite and positive")
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
chapters = args.chapters.split(",")
if any(value not in tuple(str(i) for i in range(6)) for value in chapters):
    parser.error("--chapters must contain only zero-based integers 0..5")
max_training = int(
    re.search(
        r"pub const MAX_UPGRADE_LEVEL: u8 = (\d+);",
        (ROOT / "src/game/campaign_progress.rs").read_text(),
    ).group(1)
)
training = args.training.split(",")
if any(
    value not in tuple(str(i) for i in range(max_training + 1)) for value in training
):
    parser.error(f"--training must contain integers 0..{max_training}")
chapters = [int(value) for value in chapters]
training = [int(value) for value in training]
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
if output.exists() and any(output.iterdir()):
    parser.error("--batch must name a fresh directory; existing receipts are preserved")
output.mkdir(parents=True, exist_ok=True)
# Execute one captured build throughout the batch, even if Cargo replaces its source.
source_binary = binary
snapshot = output / "simulator-snapshot"
shutil.copy2(source_binary, snapshot)
binary = snapshot.resolve()
digest = hashlib.sha256()
with binary.open("rb") as handle:
    for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
        digest.update(chunk)
binary_sha256 = digest.hexdigest()
manifest = {
    "binary": str(Path(args.binary).resolve()),
    "executed_binary": str(binary),
    "binary_sha256": binary_sha256,
    "require_telemetry": args.require_telemetry,
    "pressure_override": pressure,
    "frames": args.frames,
    "jobs": args.jobs,
    "case_timeout": args.case_timeout,
    "chapters": chapters,
    "training": training,
    "max_training_level": max_training,
    "fixed_step_secs": 1 / 60,
    "cases": [],
    "limitations": [
        "Stationary policies; active greedy card zero is not an optimal strategy.",
        "Frame cap includes menu pause frames; compare actual elapsed time.",
        "Old baseline damage-skill telemetry is unavailable and reported null.",
    ],
}
(output / "summary.json").write_text(json.dumps(manifest, indent=2))


def run_case(identity):
    chapter, trained, seed, starter, position, policy = identity
    name = f"{policy}-h{starter}-{position}-s{seed}-c{chapter}-t{trained}"
    sandbox = output / (name + "-state")
    if sandbox.exists():
        shutil.rmtree(sandbox)
    sandbox.mkdir()
    env = os.environ.copy()
    env.update(
        XDG_CONFIG_HOME=str(sandbox / "config"),
        ROGUE_CAMPAIGN_SAVE=str(sandbox / "campaign.txt"),
        ROGUE_DETAIL_SAVE=str(sandbox / "detail.txt"),
        ROGUE_SIM_CHAPTER=str(chapter),
        ROGUE_SIM_TRAINING=str(trained),
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
                    "chapter": chapter,
                    "training": trained,
                }
            )
            + "\n"
        )
        out.flush()
        timed_out = False
        try:
            result = subprocess.run(
                [str(binary), policy, seed, starter, position, str(args.frames)]
                + pressure,
                cwd=ROOT,
                env=env,
                stdout=out,
                stderr=err,
                timeout=args.case_timeout,
            )
            returncode = result.returncode
        except subprocess.TimeoutExpired:
            timed_out = True
            returncode = None
            err.write(f"Case exceeded wall timeout {args.case_timeout}s\n")
    records = []
    for line in log.read_text().splitlines():
        try:
            records.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    checkpoints = [r for r in records if r.get("type") == "checkpoint"]
    case = {
        "name": name,
        "chapter": chapter,
        "training": trained,
        "binary_sha256": binary_sha256,
        "returncode": returncode,
        "timed_out": timed_out,
        "runtime_secs": round(time.monotonic() - t, 2),
        "log": str(log),
        "stderr": str(error),
        "checkpoints": len(checkpoints),
        "last": checkpoints[-1] if checkpoints else None,
    }
    last = case["last"]
    metadata = next((r for r in records if r.get("type") == "metadata"), {})
    case["dimension_valid"] = (
        metadata.get("chapter") == chapter
        and metadata.get("training") == trained
        and bool(checkpoints)
        and all(
            r.get("actual_chapter") == chapter
            and r.get("permanent_upgrades") == [trained] * 6
            for r in checkpoints
        )
    )
    case["telemetry_valid"] = not args.require_telemetry or (
        bool(checkpoints)
        and all((r.get("damage_skill_casts") is not None for r in checkpoints))
    )
    case["valid"] = (
        returncode == 0
        and bool(checkpoints)
        and case["telemetry_valid"]
        and case["dimension_valid"]
    )
    case["outcome"] = (
        "win"
        if last and last.get("finished") and last.get("victory")
        else "loss"
        if last and last.get("finished")
        else "input_blocked"
        if last and last.get("paused")
        else "frame_cap"
    )
    if not case["valid"]:
        case["outcome"] = "invalid_receipt"
    case["censored"] = not bool(last and last.get("finished")) or not case["valid"]
    return case


def save_manifest():
    manifest["cases"].sort(key=lambda case: case["name"])
    (output / "summary.json").write_text(json.dumps(manifest, indent=2))


identities = list(
    itertools.product(
        dict.fromkeys(chapters),
        dict.fromkeys(training),
        dict.fromkeys(seeds),
        dict.fromkeys(starters),
        dict.fromkeys(positions),
        dict.fromkeys(policies),
    )
)
manifest["requested_cases"] = len(identities)
save_manifest()
failed = False
with ThreadPoolExecutor(max_workers=args.jobs) as executor:
    futures = {executor.submit(run_case, identity): identity for identity in identities}
    for future in as_completed(futures):
        if future.cancelled():
            continue
        try:
            case = future.result()
        except Exception as error:
            chapter, trained, seed, starter, position, policy = futures[future]
            case = {
                "name": f"{policy}-h{starter}-{position}-s{seed}-c{chapter}-t{trained}",
                "chapter": chapter,
                "training": trained,
                "valid": False,
                "outcome": "worker_error",
                "error": str(error),
            }
        manifest["cases"].append(case)
        save_manifest()
        print("BALANCE_DONE " + json.dumps(case), flush=True)
        if not case["valid"]:
            failed = True
            for queued in futures:
                queued.cancel()
if failed:
    raise SystemExit(
        "Balance matrix failed; invalid/timeout evidence retained in summary.json and case logs"
    )
print(f"BALANCE_MATRIX_COMPLETE {len(manifest['cases'])}", flush=True)
