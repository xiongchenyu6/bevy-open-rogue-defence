#!/usr/bin/env python3
"""Compare production-camera traces from the isolated native presentation probe."""
import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
report = {}
for speed, scenario in [('1x', 'hit-feedback'), ('8x', 'hit-feedback-8x')]:
    values = {}
    for label, name in [('before', f'before-{speed}'), ('after', scenario)]:
        text = (root / f'{name}.log').read_text()
        assert 'panicked' not in text and 'Encountered a panic' not in text
        assert 'all 12 screenshot(s) saved' in text
        assert f'wrote 180 simulation frames' in text
        assert f'PASS heavy-kill sequence leaves requested {speed[:-1]}x battle speed' in text
        rows = {}
        for frame, trauma, xyz in re.findall(r'volley-shake frame=(\d+) trauma=([\d.]+) offset=Vec3\(([^)]+)\)', text):
            offset = [float(value) for value in xyz.split(',')]
            rows[int(frame)] = dict(trauma=float(trauma), offset=offset)
        for frame in [10, 120]:
            magnitude = sum(value * value for value in rows[frame]['offset']) ** .5
            if label == 'before':
                assert magnitude == 0, (speed, label, frame, magnitude)
            else:
                assert magnitude > .0001, (speed, label, frame, magnitude)
                assert rows[frame]['trauma'] <= .07
        assert all(value == 0 for value in rows[165]['offset'])
        values[label] = rows
    report[speed] = values
print(json.dumps(report, indent=2))
