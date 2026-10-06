#!/usr/bin/env python3
"""Strict replay receipts: equal truncated prefixes are not completed runs."""

import argparse
import json
import math
import re
from pathlib import Path

IDENTITY = (
    "chapter",
    "training",
    "policy",
    "seed",
    "starter",
    "forward",
    "frame_cap",
    "fixed_step_secs",
    "executor",
)


PRESSURE_FIELDS = (
    "movement_growth",
    "attack_growth",
    "movement_cap",
    "attack_cap",
    "health_growth",
    "health_cap",
)


CHECKPOINT_FIELDS = (
    "actual_chapter",
    "permanent_upgrades",
    "actual_run_seed",
    "actual_hero_id",
    "elapsed",
    "wave",
    "level",
    "kills",
    "hero_hp",
    "max_hero_hp",
    "deaths",
    "gate_hp",
    "attacks",
    "damage_skill_casts",
    "finished",
    "victory",
)
COMPARISON_SCOPE = "recorded checkpoints, not all internal simulation state"


def reject_constant(value):
    raise ValueError(f"Non-finite JSON number: {value}")


def finite_float(value):
    number = float(value)
    if not math.isfinite(number):
        reject_constant(value)
    return number


def read_receipt(path):
    records = []
    for line_number, line in enumerate(path.read_text().splitlines(), 1):
        try:
            record = json.loads(
                line, parse_constant=reject_constant, parse_float=finite_float
            )
        except json.JSONDecodeError as error:
            if line.lstrip().startswith(("{", "[")):
                raise ValueError(
                    f"{path}: malformed JSON record on line {line_number}: {error}"
                ) from error
            continue
        except ValueError as error:
            raise ValueError(
                f"{path}: invalid JSON number on line {line_number}: {error}"
            ) from error
        if isinstance(record, dict):
            records.append(record)
        elif line.lstrip().startswith(("{", "[")):
            raise ValueError(
                f"{path}: JSONL record on line {line_number} must be an object"
            )

    def one(kind):
        matches = [record for record in records if record.get("type") == kind]
        if len(matches) != 1:
            raise ValueError(
                f"{path}: expected exactly one {kind} record, found {len(matches)}"
            )
        return matches[0]

    runner, metadata, pressure = (
        one("runner_metadata"),
        one("metadata"),
        one("pressure"),
    )
    digest = runner.get("binary_sha256")
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise ValueError(f"{path}: missing or invalid runner_metadata.binary_sha256")
    for field in IDENTITY:
        if field not in metadata or metadata[field] is None:
            raise ValueError(f"{path}: missing metadata.{field}")
    for field in PRESSURE_FIELDS:
        if field not in pressure or pressure[field] is None:
            raise ValueError(f"{path}: missing pressure.{field}")
    checkpoints = [record for record in records if record.get("type") == "checkpoint"]
    if not checkpoints:
        raise ValueError(f"{path}: no checkpoints")
    previous = -1
    for index, checkpoint in enumerate(checkpoints):
        for field in CHECKPOINT_FIELDS:
            if field not in checkpoint or checkpoint[field] is None:
                raise ValueError(f"{path}: missing checkpoint[{index}].{field}")
        upgrades = checkpoint["permanent_upgrades"]
        if (
            not isinstance(upgrades, list)
            or len(upgrades) != 6
            or any(type(value) is not int or value < 0 for value in upgrades)
        ):
            raise ValueError(
                f"{path}: checkpoint[{index}] has invalid permanent_upgrades"
            )
        for field in CHECKPOINT_FIELDS:
            if field not in ("permanent_upgrades", "finished", "victory") and type(
                checkpoint[field]
            ) not in (int, float):
                raise ValueError(f"{path}: checkpoint[{index}] has non-numeric {field}")
        if type(checkpoint["victory"]) is not bool:
            raise ValueError(f"{path}: checkpoint[{index}] has invalid victory field")
        frame = checkpoint.get("frame")
        if type(frame) is not int or frame < 0 or frame <= previous:
            raise ValueError(
                f"{path}: checkpoint[{index}] has missing, invalid or unordered frame"
            )
        if type(checkpoint.get("finished")) is not bool:
            raise ValueError(
                f"{path}: checkpoint[{index}] has missing or invalid finished field"
            )
        previous = frame
    return digest, metadata, pressure, checkpoints


def difference(left, right, field):
    if type(left) is not type(right):
        return {
            "field": field,
            "left": left,
            "right": right,
            "reason": "value type differs",
        }
    if isinstance(left, dict):
        for key in sorted(set(left) | set(right)):
            if key not in left or key not in right:
                return {
                    "field": f"{field}.{key}",
                    "left_present": key in left,
                    "right_present": key in right,
                    "reason": "field missing",
                }
            found = difference(left[key], right[key], f"{field}.{key}")
            if found:
                return found
    elif isinstance(left, list):
        for index, (a, b) in enumerate(zip(left, right)):
            found = difference(a, b, f"{field}[{index}]")
            if found:
                return found
        if len(left) != len(right):
            return {
                "field": field,
                "left_count": len(left),
                "right_count": len(right),
                "reason": "array count differs",
            }
    elif left != right:
        return {"field": field, "left": left, "right": right, "reason": "value differs"}
    return None


def compare(left_path, right_path):
    left_sha, left_meta, left_pressure, left_rows = read_receipt(left_path)
    right_sha, right_meta, right_pressure, right_rows = read_receipt(right_path)
    report = {
        "comparison_scope": COMPARISON_SCOPE,
        "left": str(left_path),
        "right": str(right_path),
        "binary_sha256": left_sha,
        "checkpoint_counts": [len(left_rows), len(right_rows)],
        "prefix_equal": False,
        "complete_replay": False,
    }
    mismatch = difference(left_sha, right_sha, "runner_metadata.binary_sha256")
    if not mismatch:
        mismatch = difference(
            {key: left_meta[key] for key in IDENTITY},
            {key: right_meta[key] for key in IDENTITY},
            "metadata",
        )
    if not mismatch:
        mismatch = difference(left_pressure, right_pressure, "pressure")
    if mismatch:
        report["first_divergence"] = mismatch
        return report, 1
    for index, (left, right) in enumerate(zip(left_rows, right_rows)):
        mismatch = difference(left, right, f"checkpoint[{index}]")
        if mismatch:
            mismatch["frame"] = left["frame"]
            mismatch["right_frame"] = right["frame"]
            report["first_divergence"] = mismatch
            return report, 1
    if len(left_rows) != len(right_rows):
        extra = left_rows if len(left_rows) > len(right_rows) else right_rows
        report["first_divergence"] = {
            "field": "checkpoint_count",
            "frame": extra[min(len(left_rows), len(right_rows))]["frame"],
            "left_count": len(left_rows),
            "right_count": len(right_rows),
            "reason": "checkpoint missing",
        }
        return report, 1
    report["prefix_equal"] = True
    report["complete_replay"] = left_rows[-1]["finished"] and right_rows[-1]["finished"]
    report["status"] = (
        "complete_equal_replay"
        if report["complete_replay"]
        else "equal_truncated_prefix"
    )
    return report, 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=Path)
    parser.add_argument("right", type=Path)
    args = parser.parse_args()
    try:
        result, status = compare(args.left, args.right)
    except (ValueError, OSError) as error:
        result = {
            "comparison_scope": COMPARISON_SCOPE,
            "prefix_equal": False,
            "complete_replay": False,
            "error": str(error),
        }
        status = 2
    print(json.dumps(result, indent=2, allow_nan=False))
    raise SystemExit(status)
