#!/usr/bin/env python3
"""Reject invalid runtime chapter data before publishing a data-only update."""
import json
import math
from pathlib import Path


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate campaign field: ' + key)
        result[key] = value
    return result


def verify(path=Path('assets/data/campaign.json')):
    data = json.loads(path.read_text(), object_pairs_hook=unique_fields)
    assert set(data) == {'chapters'}, 'Unexpected campaign fields'
    assert isinstance(data['chapters'], list) and len(data['chapters']) == 6, 'Exactly six chapters are required'
    bounds = {'duration': 86400, 'health_scale': 10000, 'damage_scale': 10000, 'gold_scale': 10000}
    for number, chapter in enumerate(data['chapters'], 1):
        assert set(chapter) == set(bounds), f'Chapter {number}: missing or unexpected fields'
        for field, maximum in bounds.items():
            value = chapter[field]
            assert type(value) in (int, float) and math.isfinite(value) and 0 < value <= maximum, f'Chapter {number}: invalid {field}'
    print(f'Validated six chapter definitions ({path.stat().st_size} bytes)')


if __name__ == '__main__':
    verify()
