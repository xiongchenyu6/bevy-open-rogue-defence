#!/usr/bin/env python3
"""Analyze real GamePlugin JSONL receipts; never infer wins from censored runs."""

import argparse
import csv
import json
from collections import defaultdict
from pathlib import Path

STARTER_CLASSES = {0: "warrior", 1: "archer", 2: "mage"}
HERO_NAMES = {0: "StormSavant", 1: "EmberRanger", 2: "StoneWarden"}
METRICS = [
    "elapsed",
    "wave",
    "level",
    "kills",
    "gate_hp",
    "hero_hp",
    "deaths",
    "gold",
    "wood",
    "bosses",
    "attacks",
    "damage_skill_casts",
    "skill_ranks",
    "enemy_peak",
    "enemy_rejected",
    "gate_breaks",
    "exposed_secs",
    "min_gate_hp",
    "min_hero_hp",
    "owned_bond_ids",
    "absorbed_strength",
    "command_health_multiplier",
    "max_hero_hp",
    "paused",
    "finished",
    "victory",
]


def read_log(path):
    records = []
    for line in Path(path).read_text().splitlines():
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(record, dict):
            records.append(record)
    metadata = next((r for r in records if r.get("type") == "metadata"), {})
    provenance = next((r for r in records if r.get("type") == "runner_metadata"), {})
    metadata = dict(metadata, binary_sha256=provenance.get("binary_sha256"))
    pressure = next((r for r in records if r.get("type") == "pressure"), {})
    return (metadata, pressure, [r for r in records if r.get("type") == "checkpoint"])


def write_csv(path, rows):
    fields = list(dict.fromkeys((key for row in rows for key in row)))
    with path.open("w", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def outcome(last):
    if last.get("finished"):
        return "win" if last.get("victory") else "loss"
    return "input_blocked" if last.get("paused") else "frame_cap"


def analyze(batches, destination, plot=False):
    cases, curves, groups = ([], [], defaultdict(list))
    for batch in batches:
        batch = Path(batch)
        summary_path = batch / "summary.json"
        summary = json.loads(summary_path.read_text()) if summary_path.exists() else {}
        receipts = summary.get("cases", [])
        logs = [
            (r.get("name", Path(r["log"]).stem), Path(r["log"]), r) for r in receipts
        ]
        if not logs:
            logs = [(p.stem, p, {}) for p in sorted(batch.glob("*.jsonl"))]
        for name, log, receipt in logs:
            if not log.is_absolute() and (not log.exists()):
                log = batch / log.name
            metadata, pressure, checkpoints = read_log(log)
            if not checkpoints:
                cases.append(
                    {
                        "batch": batch.name,
                        "case": name,
                        "outcome": "no_checkpoint",
                        "log": str(log),
                    }
                )
                continue
            first, last = (checkpoints[0], checkpoints[-1])
            identity = {
                "batch": batch.name,
                "case": name,
                "policy": metadata.get("policy", ""),
                "binary_sha256": metadata.get("binary_sha256")
                or receipt.get("binary_sha256"),
                "executor": metadata.get("executor", "production-default"),
                "requested_seed": metadata.get("seed"),
                "actual_run_seed": first.get("actual_run_seed"),
                "starter_class": STARTER_CLASSES.get(
                    metadata.get("starter"), "unknown"
                ),
                "initial_hero_id": first.get("actual_hero_id"),
                "initial_hero": HERO_NAMES.get(first.get("actual_hero_id"), "unknown"),
                "position": "forward" if metadata.get("forward") else "behind",
            }
            valid = receipt.get("valid", True)
            row = dict(
                identity,
                valid=valid,
                outcome=outcome(last) if valid else "invalid_receipt",
                censored=not last.get("finished", False) or not valid,
                final_frame=last.get("frame"),
                frame_cap=metadata.get("frame_cap"),
                fixed_step_secs=metadata.get("fixed_step_secs"),
                runtime_secs=receipt.get("runtime_secs"),
                log=str(log),
            )
            row.update({key: last.get(key) for key in METRICS})
            row["observed_min_gate_hp"] = min((r["gate_hp"] for r in checkpoints))
            row["observed_min_hero_hp"] = min((r["hero_hp"] for r in checkpoints))
            row.update({key: value for key, value in pressure.items() if key != "type"})
            cases.append(row)
            if valid:
                groups[
                    batch.name,
                    identity["policy"],
                    identity["position"],
                    identity["starter_class"],
                ].append(row)
            previous = None
            for checkpoint in checkpoints:
                curve = dict(identity, frame=checkpoint.get("frame"))
                curve.update({key: checkpoint.get(key) for key in METRICS})
                delta = checkpoint["elapsed"] - previous["elapsed"] if previous else 0
                for metric in ["attacks", "damage_skill_casts", "kills"]:
                    now = checkpoint.get(metric)
                    old = previous.get(metric) if previous else None
                    curve[metric + "_per_game_second"] = (
                        (now - old) / delta
                        if delta > 0 and now is not None and (old is not None)
                        else None
                    )
                curves.append(curve)
                previous = checkpoint
    destination.mkdir(parents=True, exist_ok=True)
    write_csv(destination / "cases.csv", cases)
    write_csv(destination / "curves-30s.csv", curves)
    aggregates = []
    for (batch, policy, position, starter), rows in sorted(groups.items()):
        wins = sum((r["outcome"] == "win" for r in rows))
        losses = sum((r["outcome"] == "loss" for r in rows))
        aggregates.append(
            {
                "batch": batch,
                "policy": policy,
                "position": position,
                "starter_class": starter,
                "cases": len(rows),
                "wins": wins,
                "losses": losses,
                "censored": len(rows) - wins - losses,
                "finished_only_win_fraction": wins / (wins + losses)
                if wins + losses
                else None,
                "observed_wins_per_all_cases": wins / len(rows),
                "mean_observed_game_seconds": sum((r["elapsed"] for r in rows))
                / len(rows),
            }
        )
    write_csv(destination / "aggregates.csv", aggregates)
    (destination / "analysis.json").write_text(
        json.dumps(
            {
                "cases": len(cases),
                "aggregates": aggregates,
                "limitations": [
                    "Censored/frame-cap/input-blocked runs are not wins.",
                    "Finished-only fractions exclude censored runs and are not unconditional win rates.",
                    "Menu pauses consume frame budget: use actual elapsed time, not frame cap, for comparisons.",
                    "Old baseline damage-skill cast telemetry may be null; null is not zero.",
                    "Observed checkpoint minima are coarse; min_gate_hp/min_hero_hp fields use every simulation frame.",
                    "The active greedy-card-zero stationary policy is not an optimal player strategy.",
                ],
            },
            indent=2,
        )
    )
    if plot:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt

        figure, axes = plt.subplots(2, 2, figsize=(12, 8), sharex=True)
        series = defaultdict(list)
        for curve in curves:
            series[curve["batch"], curve["case"]].append(curve)
        for axis, metric in zip(
            axes.flat, ["gate_hp", "hero_hp", "level", "damage_skill_casts"]
        ):
            for (batch, case), rows in series.items():
                points = [
                    (r["elapsed"] / 60, r[metric])
                    for r in rows
                    if r.get(metric) is not None
                ]
                if points:
                    axis.plot(
                        *zip(*points), alpha=0.65, linewidth=1, label=f"{batch}:{case}"
                    )
            axis.set_ylabel(metric.replace("_", " "))
            axis.grid(alpha=0.25)
        for axis in axes[1]:
            axis.set_xlabel("Actual game minutes (menu pauses excluded)")
        if len(series) <= 12:
            axes[0, 0].legend(fontsize=6)
        figure.suptitle(
            "Real GamePlugin bounded simulations; frame-cap cases are censored"
        )
        figure.tight_layout()
        figure.savefig(destination / "balance-curves.png", dpi=160)
        figure.savefig(destination / "balance-curves.svg")
        plt.close(figure)
    print(
        json.dumps(
            {
                "output": str(destination),
                "cases": len(cases),
                "checkpoints": len(curves),
            }
        )
    )


def compare(paths):
    left = read_log(paths[0])[2]
    right = read_log(paths[1])[2]
    result = {
        "left": paths[0],
        "right": paths[1],
        "checkpoints": [len(left), len(right)],
        "identical": left == right,
    }
    for index, (a, b) in enumerate(zip(left, right)):
        if a != b:
            result["first_divergence"] = {
                "checkpoint": index,
                "frame": a.get("frame"),
                "differences": {
                    key: [a.get(key), b.get(key)]
                    for key in set(a) | set(b)
                    if a.get(key) != b.get(key)
                },
            }
            break
    print(json.dumps(result, indent=2))
    return result["identical"]


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("batches", nargs="*")
    parser.add_argument("--output", default="target/verification/balance-analysis")
    parser.add_argument("--compare", nargs=2)
    parser.add_argument(
        "--plot",
        action="store_true",
        help="Optional standard matplotlib PNG/SVG charts",
    )
    args = parser.parse_args()
    if args.compare:
        raise SystemExit(0 if compare(args.compare) else 1)
    if not args.batches:
        parser.error("provide batch directories or --compare two JSONL logs")
    analyze(args.batches, Path(args.output), args.plot)
