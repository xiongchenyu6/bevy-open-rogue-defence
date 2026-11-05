#!/usr/bin/env python3
"""Validate a real-window probe's raw samples and workload invariants."""
import argparse
import csv
import json
import math
from pathlib import Path


def verify(prefix, minimum_seconds, present_mode='Fifo', surface='1280x720'):
    prefix = str(prefix)
    summary = json.loads(Path(prefix + '.json').read_text())
    with Path(prefix + '-frames.csv').open() as handle:
        intervals = [float(row['present_interval_ms']) for row in csv.DictReader(handle)]
    with Path(prefix + '-entities.csv').open() as handle:
        entities = list(csv.DictReader(handle))
    assert intervals and all(math.isfinite(value) and value > 0 for value in intervals)
    assert len(intervals) == summary['sample_intervals']
    assert summary['sample_present_frames'] == len(intervals) + 1
    assert summary['sample_wall_seconds'] >= minimum_seconds
    assert summary['assets_ready'] and summary['assets_loaded'] == summary['assets_total']
    assert summary['damage_observed'] and summary['projectiles_observed']
    assert len(summary['sample_skill_casts']) == 6 and sum(summary['sample_skill_casts']) > 0
    assert summary['speed'] in (1, 32)
    assert summary['build'] and 'unrecorded' not in summary['build'].lower()
    assert summary['surface'].split()[0] == surface and present_mode in summary['surface']
    assert entities
    for row in entities:
        assert int(row['enemies']) == int(row['imported_enemy_roots']) == 180
        assert row['paused'] == row['finished'] == 'false'
        assert float(row['actual_virtual_speed']) == summary['speed']
    ordered = sorted(intervals)
    for key, fraction in [('p50_ms', .50), ('p95_ms', .95), ('p99_ms', .99)]:
        expected = ordered[max(0, math.ceil(len(ordered) * fraction) - 1)]
        assert math.isclose(summary[key], expected, abs_tol=1e-5), (key, expected)
    assert math.isclose(summary['max_ms'], ordered[-1], abs_tol=1e-5)
    return {key: summary[key] for key in (
        'speed', 'surface', 'adapter', 'sample_intervals', 'p50_ms', 'p95_ms', 'p99_ms', 'max_ms',
        'sample_skill_casts', 'damage_observed', 'projectiles_observed',
    )}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('prefix', type=Path)
    parser.add_argument('--minimum-seconds', type=float, default=0)
    parser.add_argument('--present-mode', choices=['Fifo', 'Immediate'], default='Fifo')
    parser.add_argument('--surface', default='1280x720', help='Expected physical surface, e.g. 1920x1080')
    args = parser.parse_args()
    print(json.dumps(verify(args.prefix, args.minimum_seconds, args.present_mode, args.surface), indent=2))
