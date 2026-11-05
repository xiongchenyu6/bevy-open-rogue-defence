#!/usr/bin/env python3
"""Check paired production damage/playback observations, not implementation math."""
import ast
import json
from pathlib import Path
import re

folder = Path(__file__).resolve().parent
audit = json.loads((folder / 'clip-audit.json').read_text())
duration = audit['models'][0]['duration_seconds']


def observations(case):
    text = (folder / f'{case}.log').read_text()
    assert 'panicked' not in text and 'PASS: real basic attack damage' in text
    assert 'wrote 210 simulation frames' in text
    return [
        (int(frame), float(health), float(damage), ast.literal_eval(clips))
        for frame, health, damage, clips in re.findall(
            r'ATTACK_TIMING frame=(\d+) health=([\d.]+) damage=([\d.]+) clips=(\[.*\])', text
        )
    ]


before = [row for row in observations('before-sword') if row[2] > 0]
after = [row for row in observations('after-sword') if row[2] > 0]
assert len(before) == len(after) == 6
assert [row[2] for row in before] == [row[2] for row in after]
assert [row[1] for row in before] == [row[1] for row in after]
assert all(new[0] - old[0] == 3 for old, new in zip(before, after))
for rows, lower, upper in [(before, 0.40, 0.50), (after, 0.60, 0.80)]:
    for row in rows:
        assert len(row[3]) == 1 and row[3][0][0] == 3, 'Actual hero Attack clip required'
        assert lower < row[3][0][1] / duration < upper
controls = {}
for weapon in ['ember', 'storm']:
    baseline, changed = observations(f'before-{weapon}'), observations(f'after-{weapon}')
    assert len(baseline) == len(changed) == 80
    assert baseline == changed, f'{weapon} damage/playback regression'
    controls[weapon] = '80 production observations unchanged'
solar = [row for row in observations('after-solar') if row[2] > 0]
assert solar and all(0.60 < row[3][0][1] / duration < 0.80 for row in solar)
result = {
    'sword_hits': len(after),
    'damage_and_intervals_unchanged': True,
    'contact_shift_frames_at_30hz': 3,
    'before_observed_phase': before[0][3][0][1] / duration,
    'after_observed_phase': after[0][3][0][1] / duration,
    'controls': controls,
    'solar_contact_checked': True,
}
print(json.dumps(result, indent=2))
