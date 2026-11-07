#!/usr/bin/env python3
"""Independently check actual resolver traces joined to observed target HP bits."""
import argparse
import json
from pathlib import Path
import re
import struct

NUMBER = r"[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?"


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def from_bits(value):
    return struct.unpack('<f', struct.pack('<I', value))[0]


def field(line, key):
    match = re.search(rf'(?<!\w){key}=({NUMBER})(?=\s|$)', line)
    check(match, f'missing actual resolver numeric field {key}')
    return float(match.group(1))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('log', type=Path)
    parser.add_argument('--modifiers', type=int, choices=(0, 1), required=True)
    parser.add_argument('--frames', type=int, default=480)
    args = parser.parse_args()
    text = re.sub(r'\x1b\[[0-9;]*m', '', args.log.read_text())
    check(args.frames >= 451, 'run must reach450 probe checkpoint')
    check(not re.search(r'panicked at|Caught rendering error|Validation Error', text), 'native panic/render error')
    check(f'[capture] wrote {args.frames} simulation frames to ' in text, 'normal complete capture footer missing')
    check(re.search(r'\[capture\] all \d+ screenshot\(s\) saved after \d+ wait frame\(s\)', text), 'screenshots incomplete')
    check('[capture] PASS crown-detail:' in text, 'actual purchase and real draft text gates missing')
    expected_flag = 'true' if args.modifiers else 'false'
    check(f'[capture] PASS crown-damage-preview:' in text and f'modifier_ui={expected_flag};' in text, '450frame producer/HP gate missing')
    card_pattern = rf'CROWN_DAMAGE_CARD frame=(100|180) base_display=({NUMBER}) adjusted_display=(None|Some\(({NUMBER})\)) modifiers_expected=(true|false)'
    cards = list(re.finditer(card_pattern, text))
    check([int(row[1]) for row in cards] == [100, 180], 'both real text snapshots required')
    for row in cards:
        check(format(float(row[2]), '.1f') == '650.8', 'unexpected actual controlled raw card value')
        check(row[5] == expected_flag, 'modifier expectation mismatch')
        if args.modifiers:
            check(row[4] is not None and format(float(row[4]), '.1f') == '846.0', 'adjusted real text not846.0')
        else:
            check(row[3] == 'None', 'baseline unexpectedly has adjusted line')
    target = re.search(r'CROWN_DAMAGE_TARGET frame=200 actor=(\S+) .* initial_hp=50000 armor_component=false', text)
    check(target, 'real stationary target not created')
    actor = target[1]
    wyrm = re.search(r'CROWN_DAMAGE_WYRM frame=(\d+) public_player_summon=true actual_scene4=true imported_mesh=true', text)
    check(wyrm and 200 < int(wyrm[1]) <= 450, 'actual imported summon witness missing by450')
    intervals = {}
    hp_pattern = rf'CROWN_DAMAGE_HP frame=(\d+) actor=(\S+) hp_before=({NUMBER}) hp_after=({NUMBER}) before_bits=(\d+) after_bits=(\d+)'
    for row in re.finditer(hp_pattern, text):
        if row[2] == actor and 200 < int(row[1]) <= 450:
            intervals[(int(row[5]), int(row[6]))] = int(row[1])
    check(intervals, 'no actual before450 HP intervals')
    matched = []
    critical_count = 0
    for line in text.splitlines():
        if 'PROJECTILE_HIT' not in line:
            continue
        enemy = re.search(r'(?<!\w)enemy=(\S+)', line)
        if not enemy or enemy[1] != actor:
            continue
        profile = re.search(r'profile=DamageProfile \{([^}]+)\}', line)
        check(profile and re.search(r'element:\s*Frost\b', profile[1]) and re.search(r'class:\s*Summon\b', profile[1]) and re.search(r'skill_index:\s*Some\(26\)', profile[1]), 'actual target damage has wrong source profile')
        multiplier = field(line, 'crit_multiplier')
        critical_count += multiplier > 1.0
        pair = (int(field(line, 'hp_before_bits')), int(field(line, 'hp_after_bits')))
        if multiplier != 1.0 or pair not in intervals:
            continue
        base = f32(field(line, 'base_damage'))
        expected = f32(base * f32(1.30))
        actual = field(line, 'hp_damage')
        check(format(base, '.1f') == '650.8', 'real producer raw base differs from card')
        check(abs(actual - expected) < 0.001, 'actual noncritical resolver damage is not Crown-only1.30')
        check(struct.unpack('<I', struct.pack('<f', f32(from_bits(pair[0]) - expected)))[0] == pair[1], 'HP bits do not match actual damage subtraction')
        check(format(expected, '.1f') == '846.0', 'actual outgoing rounding differs from adjusted prediction')
        matched.append({'observed_hp_frame': intervals[pair], 'actor': actor, 'profile': 'Frost/Summon/Some26',
                        'critical_multiplier': multiplier, 'actual_base': base, 'actual_hp_damage': actual,
                        'expected_crown_only': expected, 'hp_before_bits': pair[0], 'hp_after_bits': pair[1]})
    check(matched, 'no exact real noncritical profile26 trace+HP witness by450')
    print(json.dumps({'status': 'PASS', 'modifier_ui': bool(args.modifiers), 'matched_noncritical': matched,
                      'observed_target_critical_rows': critical_count,
                      'scope': 'Actual producer/profile/HP-bit join, not physical player input or naturally earned progression. Driver process exit0/source pins must be checked separately.'}, indent=2))


if __name__ == '__main__':
    main()
