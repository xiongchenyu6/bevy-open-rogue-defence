#!/usr/bin/env python3
"""Analyze serialized offscreen update/GPU waits; these are not windowed FPS."""
import argparse
import csv
import json
import math
from pathlib import Path


def summarize(path, warmup=120, excluded=()):
    with Path(path).open() as source:
        rows = list(csv.DictReader(source))
    if not rows:
        raise ValueError('No recorded frames')
    parsed = []
    for expected, row in enumerate(rows):
        frame = int(row['frame'])
        elapsed = float(row['update_and_gpu_wait_ms'])
        counts = {name: int(row[name]) for name in ('entities', 'enemies', 'projectiles')}
        if frame != expected:
            raise ValueError(f'Non-contiguous frames: expected {expected}, found {frame}')
        if not math.isfinite(elapsed) or elapsed < 0 or min(counts.values()) < 0:
            raise ValueError(f'Invalid timing/count at frame {frame}')
        parsed.append((frame, elapsed, counts))
    values = sorted(ms for frame, ms, _ in parsed if frame >= warmup and frame not in excluded)
    if not values:
        raise ValueError('No measured frames remain after exclusions')
    percentile = lambda fraction: values[math.ceil(fraction * len(values)) - 1]
    return {
        'measurement': 'serialized offscreen application update plus GPU wait; not windowed FPS',
        'recorded_frames': len(parsed), 'measured_frames': len(values),
        'warmup_frames': warmup, 'explicit_excluded_frames': sorted(excluded),
        'median_ms': percentile(.5), 'p95_ms': percentile(.95), 'p99_ms': percentile(.99),
        'maximum_ms': max(values),
        'frames_over_16_667_ms': sum(value > 1000 / 60 for value in values),
        'frames_over_33_333_ms': sum(value > 1000 / 30 for value in values),
        'recorded_peak_counts': {name: max(counts[name] for _, _, counts in parsed)
                                 for name in ('entities', 'enemies', 'projectiles')},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('csv', type=Path)
    parser.add_argument('--warmup', type=int, default=120)
    parser.add_argument('--exclude', default='', help='Comma-separated zero-based frames')
    args = parser.parse_args()
    if args.warmup < 0:
        parser.error('warmup must be nonnegative')
    excluded = {int(value) for value in args.exclude.split(',') if value}
    if any(value < 0 for value in excluded):
        parser.error('excluded frames must be nonnegative')
    print(json.dumps(summarize(args.csv, args.warmup, excluded), indent=2))


if __name__ == '__main__':
    main()
