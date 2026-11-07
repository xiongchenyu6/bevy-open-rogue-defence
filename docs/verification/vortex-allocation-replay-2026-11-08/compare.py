"""Analysis only: no subprocesses/builds/simulations; preserve all recorded bits."""
import argparse
import itertools
import json
import pathlib
import re

ACTORS = {"enemies": 7, "projectiles": 11, "hero_positions": 4}
CONTACT_IDENTITIES = {"projectile", "projectile_bits", "enemy", "enemy_bits"}
ANSI = re.compile(r"\x1b\[[0-9;]*m")


def reject_constant(value):
    raise ValueError(f"Nonfinite JSON: {value}")


def traces(path):
    count = 0
    metadata, last_checkpoint = None, None
    with pathlib.Path(path).open() as source:
        for line in source:
            if not line.startswith("{"):
                continue
            row = json.loads(line, parse_constant=reject_constant)
            if row.get("type") == "metadata":
                metadata = row
            elif row.get("type") == "checkpoint":
                last_checkpoint = row
            if row.get("type") != "trace":
                continue
            count += 1
            if row["frame"] != count:
                raise ValueError(f"{path}: trace must cover every frame from1; expected{count}")
            if "enemy_order" not in row:
                raise ValueError("Missing raw enemy_order")
            for field, width in ACTORS.items():
                if field not in row or any(len(actor) != width for actor in row[field]):
                    raise ValueError(f"{path}: missing/changed actor schema {field}")
            yield row
    if count == 0:
        raise ValueError(f"{path}: no actual actor trace")
    if metadata is None or last_checkpoint is None or last_checkpoint["frame"] != count:
        raise ValueError(f"{path}: trace does not cover final checkpoint")
    if not last_checkpoint["finished"]:
        raise ValueError(f"{path}: unfinished or censored case; require actual terminal")


def numeric_projection(row):
    result = {key: value for key, value in row.items() if key != "enemy_order"}
    for field in ACTORS:
        # A multiset, not a set: retain every duplicate and ALL fields after ID.
        result[field] = sorted(actor[1:] for actor in row[field])
    return result


def differences(a, b):
    return {key: {"a": a.get(key), "b": b.get(key),
                  "a_present": key in a, "b_present": key in b}
            for key in sorted(set(a) | set(b))
            if key not in a or key not in b or a[key] != b[key]}


def contacts(path):
    with pathlib.Path(path).open() as source:
        for line in source:
            if "PROJECTILE_HIT " not in line:
                continue
            body = ANSI.sub("", line.rstrip()).split("PROJECTILE_HIT ", 1)[1]
            fields = list(re.finditer(r"(?<!\S)([a-z_]+)=", body))
            event = {match[1]: body[match.end():fields[i + 1].start() if i + 1 < len(fields) else len(body)].strip()
                     for i, match in enumerate(fields)}
            required = {"elapsed_bits", "enemy_bits", "profile", "hp_before_bits",
                        "hp_after_bits", "critical_roll_bits", "rng_before_crit", "rng_after_resolution"}
            if not required <= event.keys():
                raise ValueError(f"{path}: incomplete actual contact: {event}")
            # Text preserves exact decimal formatting/bit values; no float casts,
            # truncated Debug Entity prefixes, or unrecognized-field omissions.
            yield event


def actor_context(path, elapsed_bits):
    previous = None
    for row in traces(path):
        if row["elapsed_bits"] == elapsed_bits:
            return {"previous": previous, "current": row}
        previous = row
    return {"previous": None, "current": None}


def ordinary_rows(path):
    with pathlib.Path(path).open() as source:
        for line in source:
            if not line.startswith("{"):
                continue
            row = json.loads(line, parse_constant=reject_constant)
            if row.get("type") not in {"trace", "runner_metadata"}:
                yield row


def without_root(value):
    if isinstance(value, dict):
        return {k: without_root(v) for k, v in value.items() if k != "root"}
    if isinstance(value, list):
        return [without_root(v) for v in value]
    return value


def compare(left, right):
    report = {"scope": "all recorded scalar/actor bit fields, not all hidden game state",
              "actor_counts": [0, 0], "first_identity": None, "first_raw_order": None,
              "first_numeric": None, "first_missing_row": None,
              "contact_counts": [0, 0], "first_contact_identity": None,
              "first_contact_nonidentity": None}
    previous = None
    context = None
    for index, pair in enumerate(itertools.zip_longest(traces(left), traces(right))):
        a, b = pair
        for side, row in enumerate(pair):
            report["actor_counts"][side] += row is not None
        if a is None or b is None:
            if report["first_missing_row"] is None:
                report["first_missing_row"] = {"index_zero_based": index,
                    "a_frame": a and a["frame"], "b_frame": b and b["frame"]}
            continue
        strict = differences(a, b)
        if strict and report["first_identity"] is None:
            report["first_identity"] = {"frame": a["frame"], "fields": strict}
        if a["enemy_order"] != b["enemy_order"] and report["first_raw_order"] is None:
            report["first_raw_order"] = {"frame": a["frame"],
                "a": a["enemy_order"], "b": b["enemy_order"]}
        numeric = differences(numeric_projection(a), numeric_projection(b))
        if numeric and report["first_numeric"] is None:
            report["first_numeric"] = {"frame": a["frame"], "fields": numeric}
            context = {"previous": previous, "first": {"a": a, "b": b}}
        previous = {"a": a, "b": b}
    first_contacts = None
    for index, pair in enumerate(itertools.zip_longest(
            contacts(pathlib.Path(left).with_suffix(".stderr")),
            contacts(pathlib.Path(right).with_suffix(".stderr")))):
        a, b = pair
        for side, event in enumerate(pair):
            report["contact_counts"][side] += event is not None
        if a != b and report["first_contact_identity"] is None:
            report["first_contact_identity"] = index
        ca = a and {k: v for k, v in a.items() if k not in CONTACT_IDENTITIES}
        cb = b and {k: v for k, v in b.items() if k not in CONTACT_IDENTITIES}
        if ca != cb and report["first_contact_nonidentity"] is None:
            report["first_contact_nonidentity"] = index
            first_contacts = {"index_zero_based": index, "a": a, "b": b}
    if not all(report["contact_counts"]):
        raise ValueError("Both cases must contain actual contacts")
    if first_contacts:
        for side, path in [("a", left), ("b", right)]:
            event = first_contacts[side]
            if event:
                actors = actor_context(path, int(event["elapsed_bits"]))
                target = int(event["enemy_bits"])
                first_contacts[side + "_actor_context"] = {
                    "frame": actors["current"] and actors["current"]["frame"],
                    "previous_target": next((r for r in actors["previous"]["enemies"] if r[0] == target), None)
                        if actors["previous"] else None,
                    "current_target": next((r for r in actors["current"]["enemies"] if r[0] == target), None)
                        if actors["current"] else None,
                }
    report["recorded_numeric_equal"] = report["first_numeric"] is None and report["first_missing_row"] is None
    report["ordinary_counts"] = [0, 0]
    report["first_ordinary_identity"] = None
    report["first_ordinary_numeric"] = None
    for index, (a, b) in enumerate(itertools.zip_longest(ordinary_rows(left), ordinary_rows(right))):
        report["ordinary_counts"][0] += a is not None
        report["ordinary_counts"][1] += b is not None
        if a != b and report["first_ordinary_identity"] is None:
            report["first_ordinary_identity"] = {"index": index, "a": a, "b": b}
        if without_root(a) != without_root(b) and report["first_ordinary_numeric"] is None:
            report["first_ordinary_numeric"] = {"index": index, "a": a, "b": b}
    report["projection_scope"] = {
        "actors": "remove raw enemy_order and first Entity ID from each actor tuple; retain all remaining fields and multiset multiplicity",
        "contacts_excluded": sorted(CONTACT_IDENTITIES),
        "ordinary_excluded": ["root identity", "runner_metadata record"],
        "ordinals": "both outer_ordinal and candidate_ordinal retained",
        "terminal": "both final checkpoints must be finished, no capped-prefix success"}
    report["full_recorded_match"] = (report["recorded_numeric_equal"]
        and report["first_contact_nonidentity"] is None
        and report["first_ordinary_numeric"] is None)
    return report, context, first_contacts


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("left", type=pathlib.Path)
    parser.add_argument("right", type=pathlib.Path)
    parser.add_argument("output", type=pathlib.Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    if any(args.output.iterdir()):
        raise ValueError("Refuse to overwrite existing analysis evidence")
    report, actor_contexts, first_contacts = compare(args.left, args.right)
    for name, value in [("comparison", report), ("first-numeric-context", actor_contexts),
                        ("first-contact-context", first_contacts)]:
        (args.output / (name + ".json")).write_text(json.dumps(value, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("actor_counts", "contact_counts",
        "recorded_numeric_equal", "first_contact_nonidentity", "first_missing_row")}))

    raise SystemExit(0 if report["full_recorded_match"] else 1)
