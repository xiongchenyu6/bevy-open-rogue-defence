#!/usr/bin/env python3
"""Stationary public card policy plus earned, naturally stocked equipment purchases.

Run only in isolated packaged Ubuntu/X11 container. Requires CAMPAIGN_STAGE=5,
CAMPAIGN_CONTINUE=1, CAMPAIGN_SPEED=1 and PACKAGE_COMMIT. No movement claim,
stock refresh, save mutation, or resource injection. At most 12 market visits,
60 seconds apart, each bounded to 45 seconds; total battle bound 2400 seconds.
"""
import json
import os
import re
import time
import subprocess
import tempfile
from pathlib import Path
from campaign_runtime import GAME, main
from smoke_runtime import find_text, normalized, read_hud_wallet


PREVIEW_CROP = '425x19+695+489'


TILE_LEFTS = (615, 742, 868, 995, 1122)


def battle_seconds(text):
    match = re.fullmatch(r'(\d{2}):(\d{2})', text.strip())
    assert match and int(match[2]) < 60, f'Unreadable battle countdown: {text!r}'
    return int(match[1]) * 60 + int(match[2])


def screenshot_battle_seconds(path, env):
    return battle_seconds(crop_text(path, '100x28+1170+20', env))


def market_wallet(rows):
    values = {int(match[1]) for text, x, y, *_ in rows
              if 1110 <= x < 1220 and 201 <= y < 216
              and (match := re.search(r'KILLS\s*(\d+)\b', text, re.I))}
    assert len(values) <= 1, f'Ambiguous market wallet: {values}'
    return next(iter(values), None)


def has_gear_rows(rows):
    # The illustrated market has no [GEAR] paragraphs. Keep an unreadable
    # wallet from making an open panel look closed.
    return any(600 <= x < 1250 and 200 <= y < 240
               and 'blackmarket' in normalized(text) for text, x, y, *_ in rows)


def market_closed(rows):
    return market_wallet(rows) is None and not has_gear_rows(rows)


def crop_text(path, geometry, env, digits=False):
    """OCR only public screenshot pixels; never query game state."""
    with tempfile.TemporaryDirectory(prefix='rogue-market-') as directory:
        crop = Path(directory) / 'crop.png'
        subprocess.run(['convert', str(path), '-crop', geometry, '-resize', '600%',
                        '-colorspace', 'Gray', '-negate', '-bordercolor', 'white',
                        '-border', '20', str(crop)], env=env, check=True, capture_output=True)
        args = ['tesseract', str(crop), 'stdout', '-l', 'eng', '--psm', '6']
        if digits:
            args += ['-c', 'tessedit_char_whitelist=0123456789']
        return subprocess.check_output(args, env=env, text=True, stderr=subprocess.PIPE).strip()


def screenshot_wallet(path, env):
    text = crop_text(path, '95x14+1119+202', env)
    assert re.fullmatch(r'LV\d+\s+KILLS\s*\d+', text), f'Unreadable market wallet: {text!r}'
    return int(re.search(r'KILLS\s*(\d+)', text)[1]), text


def tile_name_matches(tile_text, name):
    # Name-only crop excludes price/rank/category. Real labels can wrap;
    # accept at most two contiguous alphabetic lines, never a substring or
    # a join across noisy/non-name rows.
    lines = tile_text.splitlines()
    expected = normalized(name)
    for start in range(len(lines)):
        for length in (1, 2):
            part = lines[start:start + length]
            if len(part) != length or not all(re.fullmatch(r"[A-Za-z][A-Za-z '-]*", line.strip())
                                               for line in part):
                continue
            if any(line.strip().lower() in ('equipment', 'consumable', 'r', 'sr', 'ssr', 'ur')
                   for line in part):
                continue
            if normalized(' '.join(part)) == expected:
                return True
    return False


def inspected_offer(tile_text, price_text, preview_text, index):
    assert re.fullmatch(r'\d+', price_text), f'Unreadable tile price: {price_text!r}'
    match = re.fullmatch(r"(.+?)\s*[·•*+,:.»]\s*[‘’']?(Equipment|Consumable)", preview_text)
    assert match, f'Unreadable item category: {preview_text!r}'
    name, category = match.groups()
    assert tile_name_matches(tile_text, name), \
        f'Tile/preview mismatch: {tile_text!r}, {preview_text!r}'
    return dict(name=name.strip(), price=int(price_text), category=category, index=index,
                tile_ocr=tile_text, price_ocr=price_text, preview_ocr=preview_text)


def affordable_gear(offers, wallet):
    assert len(offers) == 5 and {item['index'] for item in offers} == set(range(5)), \
        'All five natural offers must be inspected before choosing cheapest equipment'
    return sorted((item for item in offers if item['category'] == 'Equipment'
                   and 0 < item['price'] <= wallet), key=lambda item: (item['price'], item['index']))


def verify_deduction(wallet, remaining, offer):
    assert 0 < offer['price'] <= wallet and wallet - remaining == offer['price'], \
        ('Wrong purchase deduction', wallet, remaining, offer)


def inventory_count(rows):
    counts = {int(match[1]) for text, x, y, *_ in rows
              if 200 <= x < 600 and 80 <= y < 145
              and (match := re.fullmatch(r'EQUIPMENT\s*[·•.:-]?\s*(\d+)', text.strip(), re.I))}
    assert len(counts) == 1, f'Actual equipment header count missing/ambiguous: {counts}'
    return counts.pop()


def inventory_icons(count):
    # Public 1280x720 visible grid, catalog order rather than purchase order.
    assert 1 <= count <= 20, f'Inventory count {count} exceeds verified visible grid bound 1..20'
    return [(245 + 86 * (index % 10), 184 + 86 * (index // 10)) for index in range(count)]


def inventory_open(rows):
    return any(200 <= x < 600 and 80 <= y < 145
               and re.match(r'^EQUIPMENT\b', text.strip(), re.I)
               for text, x, y, *_ in rows)


def selected_inventory_name(text):
    match = re.fullmatch(r'\[(?:R|SR|SSR|UR)\]\s+([A-Za-z][A-Za-z -]+)', text.strip())
    assert match, f'Unreadable selected inventory detail: {text!r}'
    return normalized(match[1])


def choice_pending(rows):
    return any(any(token in normalized(text) for token in
                   ('forgeanewbond', 'chooseyour', 'shapeyourarsenal',
                    'recruitahero', 'chooseanevolution')) for text, *_ in rows)


class EquipmentPolicy:
    def __init__(self, ui):
        assert os.environ.get('CAMPAIGN_STAGE') == '5'
        assert os.environ.get('CAMPAIGN_CONTINUE') == '1'
        assert os.environ.get('CAMPAIGN_SPEED') == '1'
        version = json.loads((GAME / 'version.json').read_text())
        assert version['git_commit'] == os.environ['PACKAGE_COMMIT']
        self.ui = ui
        self.next_visit = 185
        self.visits = 0
        self.purchases = []
        ui['report'].update(scope='Packaged public UI; stationary first-card policy plus earned equipment',
            strategy='Stationary keys 1/g/f; cheapest affordable natural gear; no refresh or movement',
            package_version=version, equipment_purchases=self.purchases,
            equipment_visit_limit=12, equipment_visit_timeout_seconds=45)

    def key(self, key):
        self.ui['call'](['xdotool', 'keydown', key])
        time.sleep(.15)
        self.ui['call'](['xdotool', 'keyup', key])
        time.sleep(.35)

    def click(self, row):
        _, x, y, right, bottom = row
        self.ui['call'](['xdotool', 'mousemove', '--window', self.ui['window'](),
                         str((x + right) // 2), str((y + bottom) // 2)])
        self.ui['call'](['xdotool', 'click', '1'])
        time.sleep(.5)

    def observe(self, rows, elapsed):
        if elapsed < self.next_visit or self.visits >= 12:
            return
        self.visits += 1
        self.next_visit = elapsed + 60
        deadline = time.monotonic() + 45
        prefix = f'equipment-{self.visits:02}'
        with self.ui['input_lock']:
            # Drain only already queued choices; never draw or refresh here.
            for _ in range(20):
                rows = self.ui['lines']()
                if choice_pending(rows):
                    self.key('1')
                else:
                    break
                assert time.monotonic() < deadline, 'Existing card queue exceeded market visit budget'
            assert not choice_pending(self.ui['lines']()), 'Existing choice queue not drained after 20 selections'
            self.key('b')
            rows = self.ui['lines']()
            self.ui['report'].setdefault('equipment_visit_rows', []).append(dict(visit=self.visits, rows=rows))
            if not has_gear_rows(rows):
                assert market_wallet(rows) is None, 'Market visibility ambiguous'
                self.ui['shot'](prefix + '-unavailable.png')
                return
            market = self.ui['shot'](prefix + '-market.png')
            wallet, wallet_raw = screenshot_wallet(market, self.ui['env'])
            inspected = []
            for index, left in enumerate(TILE_LEFTS):
                assert time.monotonic() < deadline, 'Offer inspection exceeded visit budget'
                # Hover only: clicking a desktop tile buys immediately.
                self.ui['call'](['xdotool', 'mousemove', '--window', self.ui['window'](),
                                 str(left + 60), '365'])
                time.sleep(.2)
                image = self.ui['shot'](prefix + f'-inspect-{index}.png')
                tile = crop_text(image, f'120x40+{left}+343', self.ui['env'])
                price = crop_text(image, f'39x23+{left+52}+384', self.ui['env'], digits=True)
                preview = crop_text(image, PREVIEW_CROP, self.ui['env'])
                inspected.append(inspected_offer(tile, price, preview, index))
            self.ui['report'].setdefault('equipment_inspections', []).append(
                dict(visit=self.visits, wallet=wallet, wallet_ocr=wallet_raw, offers=inspected))
            offers = affordable_gear(inspected, wallet)
            if offers:
                offer = offers[0]
                before = self.ui['shot'](prefix + '-before.png')
                hud, _, _ = read_hud_wallet(before, self.ui['env'])
                assert hud == wallet, ('Market/HUD wallet disagreement', hud, wallet)
                left = TILE_LEFTS[offer['index']]
                self.ui['call'](['xdotool', 'mousemove', '--window', self.ui['window'](), str(left+60), '365'])
                time.sleep(.2)
                selected = self.ui['shot'](prefix + '-selected.png')
                checked = inspected_offer(crop_text(selected, f'120x40+{left}+343', self.ui['env']),
                    crop_text(selected, f'39x23+{left+52}+384', self.ui['env'], digits=True),
                    crop_text(selected, PREVIEW_CROP, self.ui['env']), offer['index'])
                assert checked == offer, 'Selected offer changed before purchase'
                self.click(('', 1129, 482, 1239, 530))
                after = self.ui['shot'](prefix + '-after.png')
                remaining, _, raw = read_hud_wallet(after, self.ui['env'])
                verify_deduction(wallet, remaining, offer)
                purchase = dict(name=offer['name'], price=offer['price'], wallet_before=wallet,
                                wallet_after=remaining, offer_ocr=offer, after_wallet_ocr=raw,
                                visit=self.visits, before=before.name, after=after.name)
                self.purchases.append(purchase)
            self.key('b')
            while time.monotonic() < deadline:
                closed_rows = self.ui['lines']()
                if market_closed(closed_rows):
                    break
                time.sleep(.3)
            else:
                self.ui['report']['market_close_failure_rows'] = closed_rows
                self.ui['shot'](prefix + '-close-failure.png')
                raise AssertionError('Market failed to close within visit budget')
            if offers:
                self.key('i')
                inventory = self.ui['lines']()
                count = inventory_count(inventory)
                details = []
                found = None
                for index, (x, y) in enumerate(inventory_icons(count)):
                    assert time.monotonic() < deadline, 'Inventory inspection exceeded visit budget'
                    self.click(('', x, y, x, y))
                    image = self.ui['shot'](prefix + f'-inventory-icon-{index}.png')
                    text = crop_text(image, '840x25+220+445', self.ui['env'])
                    name = selected_inventory_name(text)
                    details.append(dict(icon=[x, y], detail_ocr=text))
                    if name == normalized(offer['name']):
                        found = [x, y]
                        break
                assert found is not None, 'Purchased item not found in visible selected inventory details'
                purchase['inventory_count'] = count
                purchase['selected_icon'] = found
                purchase['inventory_inspections'] = details
                inventory = self.ui['lines']()
                self.ui['shot'](prefix + '-inventory.png')
                close = find_text(inventory, 'Close')
                assert close, 'Actual equipment Close button not visible'
                self.click(close)
                closed_inventory = self.ui['lines']()
                assert not inventory_open(closed_inventory) and not find_text(closed_inventory, 'Close'), \
                    'Equipment did not close'
                purchase['inventory_verified'] = True
            # Header absence alone can be OCR failure. Prove actual simulation
            # resumes through the public countdown after both panels are closed.
            resumed = self.ui['shot'](prefix + '-resume-before.png')
            before_seconds = screenshot_battle_seconds(resumed, self.ui['env'])
            resume_deadline = min(deadline, time.monotonic() + 10)
            while time.monotonic() < resume_deadline:
                time.sleep(.5)
                rows = self.ui['lines']()
                if choice_pending(rows):
                    self.key('1')
                    continue
                resumed = self.ui['shot'](prefix + '-resume-after.png')
                after_seconds = screenshot_battle_seconds(resumed, self.ui['env'])
                if after_seconds < before_seconds:
                    self.ui['report'].setdefault('equipment_resume_proofs', []).append(
                        dict(visit=self.visits, before=before_seconds, after=after_seconds))
                    break
            else:
                raise AssertionError('Battle countdown did not resume after market/equipment closure')
            assert time.monotonic() < deadline, 'Market visit exceeded 45 seconds'
            print(json.dumps(dict(stage='earned equipment visit', visit=self.visits,
                                  purchases=len(self.purchases), wallet=wallet)), flush=True)

    def verify(self):
        assert self.purchases, 'No earned equipment purchased; active equipment route unproven'
        assert all(item.get('inventory_verified') for item in self.purchases)


if __name__ == '__main__':
    main(EquipmentPolicy)
