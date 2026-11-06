#!/usr/bin/env python3
"""Compare completed real resolver fixtures; never infer exact rendered contact frames."""
import ast
from collections import Counter, defaultdict
from decimal import Decimal, InvalidOperation
import hashlib
import json
from pathlib import Path
import re
import struct
import sys

ROOT = Path(__file__).resolve().parent
ANSI = re.compile(r"\x1b\[[0-9;]*m")
NUMBER = r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?"
CONTACT = re.compile(rf"^HIT_MATERIAL_CONTACT frame=(\d+) actor=(\S+) hp=({NUMBER}) damage=({NUMBER})$")
TIMING = re.compile(rf"^ATTACK_TIMING frame=(\d+) health=({NUMBER}) damage=({NUMBER}) clips=(.*)$")
HEADER = re.compile(r"^(?:HIT_MATERIAL(?:_CONTACT|_BASELINE)?|ATTACK_TIMING|HIT_PUNCH_FIRST) frame=(\d+)\b")
MATERIAL = re.compile(r"^HIT_MATERIAL frame=(\d+) leaf=(\S+) actor=(\S+) control=(true|false) original=(.*?) current=(.*?) changed=(true|false) base=(.*?) emissive=(.*?) alpha=(.*?) no_shadow=(true|false)$")
TRACE_FIELDS = (
    "base_damage", "scaled_base_damage", "raw_damage", "final_damage",
    "critical_roll", "critical_roll_bits", "crit_multiplier", "hp_before_bits",
    "hp_after_bits", "hp_damage", "rng_before_crit", "rng_after_crit",
    "rng_after_resolution", "elapsed_bits",
)


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def number(value):
    result = Decimal(str(value))
    require(result.is_finite(), f"nonfinite number: {value}")
    return result


def field(line, key):
    match = re.search(rf"(?<!\w){re.escape(key)}=({NUMBER})(?=\s|$)", line)
    require(match is not None, f"missing numeric trace field {key}")
    return number(match.group(1))


def rgba(text):
    # Current imported materials and variants log Srgba / LinearRgba structs.
    channels = []
    for name in ("red", "green", "blue", "alpha"):
        match = re.search(rf"\b{name}:\s*({NUMBER})", text)
        if match is None:
            return None
        channels.append(float(number(match.group(1))))
    return channels


def base_srgba(row):
    channels = row["base"]
    if channels is None:
        return None
    if row["base_format"] == "Srgba":
        return channels
    if row["base_format"] == "LinearRgba":
        # Standard linear-light -> IEC sRGB transfer. Alpha is never gamma
        # converted; StandardMaterial.emissive is already linear and stays so.
        def encode(value):
            return 12.92 * value if value <= 0.0031308 else 1.055 * value ** (1.0 / 2.4) - 0.055
        return [encode(value) for value in channels[:3]] + [channels[3]]
    return None


def f32(value):
    return struct.unpack("<f", struct.pack("<f", value))[0]


def bits_f32(value):
    return struct.unpack("<f", struct.pack("<I", int(value)))[0]


def diff_counters(left, right):
    return {"baseline_only": [list(row) + [count] for row, count in (left - right).items()][:12],
            "after_only": [list(row) + [count] for row, count in (right - left).items()][:12]}


def load(stage):
    path = ROOT / f"{stage}-real-native.log"
    require(path.is_file(), f"missing log {path}")
    text = ANSI.sub("", path.read_text())
    receipt_path = ROOT / f"{stage}-real-native-result.json"
    require(receipt_path.is_file(), f"missing process receipt {receipt_path}")
    receipt = json.loads(receipt_path.read_text())
    require(receipt.get("returncode") == 0, f"{stage}: process did not exit zero")
    command = receipt.get("command", [])
    require("frames" in command and "250" in command and "weapon-growth-sword-starter" in command,
            f"{stage}: receipt does not describe requested real250 growth fixture")
    require(not re.search(r"panicked at|thread .*panicked|Caught rendering error|Validation Error|Quitting the application due", text),
            f"{stage}: panic or rendering error")
    require(len(re.findall(r"\[capture\] wrote 250 simulation frames to ", text)) == 1,
            f"{stage}: missing unique complete250 footer")
    completed = re.findall(r"\[capture\] all 250 screenshot\(s\) saved after (\d+) wait frame\(s\)", text)
    require(len(completed) == 1, f"{stage}: screenshots not all completed")
    max_probe_frame = 250 + int(completed[0])
    gates = {
        "real_material": r"HIT_MATERIAL_PASS hits=([1-9]\d*) swapped=true restored=true source_and_control_unchanged=true",
        "real_attack": r"\[capture\] PASS: real basic attack damage and weapon visuals; learned skills/ultimate/Autoplay suppressed",
        "growth230": r"\[capture\] PASS weapon-growth-sword-starter: production attack flight\+damage",
        "material_finish": r"\[capture\] PASS material finish:",
    }
    for name, pattern in gates.items():
        require(re.search(pattern, text) is not None, f"{stage}: missing gate {name}")
    image_dir = ROOT / f"{stage}-real"
    for frame in range(250):
        image = image_dir / f"frame{frame:05}.png"
        require(image.is_file(), f"{stage}: missing actual frame {frame}")
        with image.open("rb") as stream:
            header = stream.read(24)
        require(len(header) == 24 and header[:8] == b"\x89PNG\r\n\x1a\n" and struct.unpack(">II", header[16:24]) == (1280, 720),
                f"{stage}: invalid actual1280x720 PNG {frame}")

    contacts, timing, traces, material_rows = [], [], [], []
    pending_traces = []
    for line in text.splitlines():
        if "PROJECTILE_HIT" in line:
            enemy = re.search(r"(?<!\w)enemy=(\S+)", line)
            require(enemy is not None, f"{stage}: trace enemy missing")
            values = {name: field(line, name) for name in TRACE_FIELDS}
            require(values["crit_multiplier"] >= 1, f"{stage}: invalid critical multiplier")
            row = {"actor": enemy.group(1), **values, "frame": None}
            pending_traces.append(row)
            traces.append(row)
        header = HEADER.match(line)
        if header and pending_traces:
            for row in pending_traces:
                row["frame"] = int(header.group(1))
            pending_traces.clear()
        match = CONTACT.match(line)
        if match:
            frame, actor, hp, damage = match.groups()
            contacts.append((int(frame), actor, number(hp), number(damage)))
        elif line.startswith("HIT_MATERIAL_CONTACT"):
            raise AssertionError(f"{stage}: malformed contact row")
        match = TIMING.match(line)
        if match:
            frame, health, damage, clip_text = match.groups()
            clips = ast.literal_eval(clip_text)
            require(isinstance(clips, list), f"{stage}: invalid clips")
            numeric_clips = []
            for clip in clips:
                require(isinstance(clip, tuple) and len(clip) == 3 and isinstance(clip[0], int),
                        f"{stage}: malformed numeric animation tuple")
                numeric_clips.append((clip[0], number(clip[1]), number(clip[2])))
            timing.append((int(frame), number(health), number(damage), tuple(numeric_clips)))
        elif line.startswith("ATTACK_TIMING"):
            raise AssertionError(f"{stage}: malformed attack timing row")
        match = MATERIAL.match(line)
        if match:
            frame, leaf, actor, control, original, current, changed, base, emission, alpha, shadow = match.groups()
            material_rows.append({"frame": int(frame), "leaf": leaf, "actor": actor,
                                  "control": control == "true", "original": original,
                                  "current": current, "changed": changed == "true",
                                  "base": rgba(base), "base_format": base.split("(", 1)[0],
                                  "emissive": rgba(emission), "alpha_mode": alpha,
                                  "no_shadow": shadow == "true"})
        elif line.startswith("HIT_MATERIAL frame="):
            raise AssertionError(f"{stage}: malformed material row")
    require(contacts, f"{stage}: no actual HP contacts")
    require(traces, f"{stage}: missing actual resolver trace")
    require(not pending_traces and all(row["frame"] is not None for row in traces),
            f"{stage}: resolver rows cannot be assigned to observed frame headers")
    require(all(1 <= row[0] <= max_probe_frame for row in contacts), f"{stage}: contact outside capture/wait bound")
    require([row[0] for row in timing] == list(range(1, 81)), f"{stage}: incomplete or duplicate timing1..80")
    require(material_rows and any(row["control"] for row in material_rows), f"{stage}: missing material/control rows")
    require(not any(row["control"] and row["changed"] for row in material_rows), f"{stage}: shared control mutated")
    ordinary = sum(row["crit_multiplier"] == 1 for row in traces)
    critical = sum(row["crit_multiplier"] > 1 for row in traces)
    require(ordinary and critical, f"{stage}: both actual ordinary and critical contacts required")
    return {"stage": stage, "log": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "receipt": receipt, "contacts": contacts, "timing": timing, "traces": traces,
            "materials": material_rows, "ordinary_count": ordinary, "critical_count": critical,
            "max_probe_frame": max_probe_frame}


def match_materials(run):
    by_actor_frame = defaultdict(list)
    for row in run["traces"]:
        by_actor_frame[(row["frame"], row["actor"])].append(row)
    matches, unmatched = {}, []
    for frame, actor, hp, damage in run["contacts"]:
        rows = by_actor_frame.get((frame, actor), [])
        if rows:
            end_hp = bits_f32(rows[-1]["hp_after_bits"])
            start_hp = bits_f32(rows[0]["hp_before_bits"])
            hp_match = number(format(end_hp, ".3f")) == hp
            damage_match = number(format(f32(start_hp - end_hp), ".3f")) == damage
            if hp_match and damage_match:
                matches[(frame, actor)] = any(row["crit_multiplier"] > 1 for row in rows)
                continue
        unmatched.append({"frame": frame, "actor": actor, "reason": "no exact same-run actor/frame + HP-bit damage match"})
    originals = {}
    previous = {}
    matched, unclassified = [], []
    for row in run["materials"]:
        if not row["changed"]:
            originals.setdefault(row["original"], row)
        prior = previous.get(row["leaf"])
        fresh = row["changed"] and prior is not None and (not prior["changed"] or prior["current"] != row["current"])
        previous[row["leaf"]] = row
        if not row["changed"] or row["control"]:
            continue
        key = (row["frame"], row["actor"])
        # After probe100, unchanged handles are not printed. An unlogged restore
        # is not evidence of a fresh swap. Join actual material observations on
        # the exact HP-matched contact frame instead; record visible transitions
        # separately and make no lifecycle inference from the missing rows.
        if key not in matches:
            if fresh:
                unclassified.append({"frame": row["frame"], "actor": row["actor"], "reason": "observed handle transition lacks exact same-frame contact evidence"})
            continue
        source = originals.get(row["original"])
        source_base = base_srgba(source) if source is not None else None
        actual_base = base_srgba(row)
        if source_base is None or source["emissive"] is None or actual_base is None or row["emissive"] is None:
            unclassified.append({"frame": row["frame"], "actor": row["actor"], "reason": "contact-frame material lacks supported exact source/color-space evidence"})
            continue
        critical = matches[key]
        if run["stage"] == "baseline":
            expected_base = [1.0, 1.0, 1.0, source_base[3]]
            expected_emissive = [1.0, 1.0, 1.0, source["emissive"][3]]
        else:
            amount, tint = (0.55, [1.0, 0.82, 0.38]) if critical else (0.18, [1.0, 1.0, 1.0])
            expected_base = [source_base[i] + (tint[i] - source_base[i]) * amount for i in range(3)] + [source_base[3]]
            extra = [0.4, 0.18, 0.03] if critical else [0.0, 0.0, 0.0]
            expected_emissive = [source["emissive"][i] + extra[i] for i in range(3)] + [source["emissive"][3]]
        # Emission alpha is not an opacity channel; compare only actual RGB.
        correct = all(abs(a - b) <= 0.00001 for a, b in zip(actual_base, expected_base)) and all(
            abs(a - b) <= 0.00001 for a, b in zip(row["emissive"][:3], expected_emissive[:3]))
        require(correct, f"{run['stage']}: actual contact-frame {'critical' if critical else 'ordinary'} material mismatch at frame{row['frame']}")
        matched.append({"frame": row["frame"], "actor": row["actor"], "tier": "critical" if critical else "ordinary",
                        "base_srgba": actual_base, "source_base_format": source["base_format"],
                        "emissive_linear": row["emissive"], "model_checked": True,
                        "handle_transition_observed": fresh})
    fresh_unique = {(row["frame"], row["actor"], row["tier"]) for row in matched if row["handle_transition_observed"]}
    unique = {(row["frame"], row["actor"], row["tier"]) for row in matched}
    candidates = {}
    for tier in ("ordinary", "critical"):
        frames = sorted({frame for frame, _, category in unique if category == tier})
        candidates[tier] = [{"probe_contact_frame": frame,
                             "png_candidates_only": sorted({n for offset in (-2, -1, 0, 1, 2, 3, 6) if 0 <= (n := frame + offset) < 250})}
                            for frame in frames[:6]]
    return {"matched_contact_groups": len(matches), "unmatched_contact_groups": unmatched,
            "contact_frame_material_groups": {tier: sum(row[2] == tier for row in unique) for tier in ("ordinary", "critical")},
            "fresh_material_groups": {tier: sum(row[2] == tier for row in fresh_unique) for tier in ("ordinary", "critical")},
            "checked_material_rows": matched, "unclassified_fresh_swaps": unclassified,
            "screenshot_candidates": candidates,
            "scope": "Same-run identities + frame + exact HP-bit endpoints establish the contact join. Contact-frame material observations are distinct from observed handle transitions; missing post100 original-handle logs do not prove restoration. LinearRgba base colors use standard linear-to-sRGB conversion; emissive stays linear and alpha stays unchanged. Unmatched rows do not establish a tier. PNG candidates need visual review, not exact-frame claims."}


def main():
    report = {"status": "FAIL", "analysis_revision": "r2-color-space-contact-join",
              "previous_r1_sha256": hashlib.sha256((ROOT / "comparison-r1.json").read_bytes()).hexdigest() if (ROOT / "comparison-r1.json").is_file() else None, "scope": "Real250 contact supplement only; synthetic180 evidence is separate.",
              "claim_limits": ["No screenshot pixels or actual sound are evaluated.",
                               "Contact printouts have3decimal precision; resolver fields additionally compare HP/critical/RNG bits.",
                               "Cross-run actor/projectile IDs and outer iteration order are not compared."]}
    try:
        before, after = load("baseline"), load("after")
        contacts_before = Counter((frame, hp, damage) for frame, _, hp, damage in before["contacts"])
        contacts_after = Counter((frame, hp, damage) for frame, _, hp, damage in after["contacts"])
        timing_before, timing_after = Counter(before["timing"]), Counter(after["timing"])
        resolver_before = Counter((row["frame"], *(row[key] for key in TRACE_FIELDS)) for row in before["traces"])
        resolver_after = Counter((row["frame"], *(row[key] for key in TRACE_FIELDS)) for row in after["traces"])
        report["runs"] = {run["stage"]: {"log": run["log"], "sha256": run["sha256"],
                                            "contacts": len(run["contacts"]), "timing_rows": len(run["timing"]),
                                            "resolver_rows": len(run["traces"]), "ordinary": run["ordinary_count"],
                                            "critical": run["critical_count"], "receipt": run["receipt"]}
                          for run in (before, after)}
        report["contact_multiset_equal"] = contacts_before == contacts_after
        report["attack_timing_numeric_rows_equal"] = timing_before == timing_after
        report["resolver_numeric_rows_equal"] = resolver_before == resolver_after
        report["contact_differences"] = diff_counters(contacts_before, contacts_after)
        report["timing_differences"] = diff_counters(timing_before, timing_after)
        report["resolver_differences"] = diff_counters(resolver_before, resolver_after)
        require(contacts_before == contacts_after, "per-frame HP/damage multiplicities differ")
        require(timing_before == timing_after, "actual animation/HP/damage numeric rows differ")
        require(resolver_before == resolver_after, "actual resolver critical/RNG/HP-bit numeric rows differ")
        report["material_matching"] = {run["stage"]: match_materials(run) for run in (before, after)}
        report["status"] = "PASS"
    except (AssertionError, OSError, ValueError, InvalidOperation, SyntaxError, KeyError, TypeError, OverflowError, struct.error) as error:
        report["error"] = str(error)
    (ROOT / "comparison.json").write_text(json.dumps(report, indent=2, default=str) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key not in ("material_matching", "runs")}, indent=2, default=str))
    return 0 if report["status"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
