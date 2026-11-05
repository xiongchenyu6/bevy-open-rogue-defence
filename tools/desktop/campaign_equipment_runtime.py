#!/usr/bin/env python3
"""Stationary public card policy plus earned, naturally stocked equipment purchases.

Run only in isolated packaged Ubuntu/X11 container. Requires CAMPAIGN_STAGE=5,
CAMPAIGN_CONTINUE=1, CAMPAIGN_SPEED=1 and PACKAGE_COMMIT. No movement claim,
stock refresh, save mutation, or resource injection. Visits have finite shared
inspection/resume budgets and a 60-second cooldown after completion; the report
records the visit, cumulative pause, and total battle bounds.
"""
import json
import os
import re
import time
import subprocess
import tempfile
import shutil
from pathlib import Path
from campaign_runtime import GAME, main
from smoke_runtime import find_text, normalized, read_hud_wallet, read_toolbar_speed, ocr_screenshot


PREVIEW_CROP = '425x19+695+489'


TILE_LEFTS = (615, 742, 868, 995, 1122)


def tile_name_crop(left):
    # Exclude both the item art above the caption and the tile's side borders.
    return f'108x28+{left + 6}+353'


def battle_seconds(text):
    match = re.fullmatch(r'(\d{2}):(\d{2})', text.strip())
    assert match and int(match[2]) < 60, f'Unreadable battle countdown: {text!r}'
    return int(match[1]) * 60 + int(match[2])


def battle_clock_reads(path, env):
    """Read only the public timer digits; native and enlarged glyphs may differ."""
    reads = []
    with tempfile.TemporaryDirectory(prefix='rogue-battle-clock-') as directory:
        for scale in (100, 200, 600):
            crop = Path(directory) / f'clock-{scale}.png'
            subprocess.run(['convert', str(path), '-crop', '100x28+1170+20',
                '-resize', f'{scale}%', '-colorspace', 'Gray', '-negate',
                '-bordercolor', 'white', '-border', '20', str(crop)],
                env=env, check=True, capture_output=True)
            text = subprocess.check_output(['tesseract', str(crop), 'stdout', '-l', 'eng',
                '--psm', '7', '-c', 'tessedit_char_whitelist=0123456789:'],
                env=env, text=True, stderr=subprocess.PIPE).strip()
            reads.append((scale, text))
    return reads


def screenshot_battle_seconds(path, env):
    reads = battle_clock_reads(path, env)
    complete = [(scale, battle_seconds(text)) for scale, text in reads
                if re.fullmatch(r'\d{2}:\d{2}', text)]
    values = {value for _, value in complete}
    assert len({scale for scale, _ in complete}) >= 2 and len(values) == 1, \
        f'Unreadable/ambiguous battle countdown pixels: {reads!r}'
    return values.pop()


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


def tile_name_text(path, left, env):
    """Preserve colored rarity labels using their brightest RGB channel.

    Luminance grayscale dims saturated red/blue lettering. Channel maximum
    retains the original antialiasing without guessing the equipment name or
    selecting a color threshold. Crop excludes art, rank, borders and price.
    """
    with tempfile.TemporaryDirectory(prefix='rogue-tile-name-') as directory:
        crop = Path(directory) / 'name.png'
        subprocess.run(['convert', str(path), '-crop', tile_name_crop(left),
                        '-channel', 'RGB', '-separate', '+channel',
                        '-evaluate-sequence', 'Max', '-resize', '600%', '-negate',
                        '-bordercolor', 'white', '-border', '20', str(crop)],
                       env=env, check=True, capture_output=True)
        return subprocess.check_output(['tesseract', str(crop), 'stdout', '-l', 'eng',
                                        '--psm', '6'], env=env, text=True,
                                       stderr=subprocess.PIPE).strip()


def wallet_header_reads(path, env):
    """Read the complete public header at multiple measured raster scales.

    A 600% enlargement can merge/lose tiny adjacent digits even when the
    native header is clear. Smaller 200/300% reads retain those glyphs; every
    accepted whole-line reading must agree, rather than choosing a balance.
    """
    results = []
    with tempfile.TemporaryDirectory(prefix='rogue-market-wallet-') as directory:
        for scale in (200, 300, 600):
            crop = Path(directory) / f'header-{scale}.png'
            subprocess.run(['convert', str(path), '-crop', '95x14+1119+202',
                            '-resize', f'{scale}%', '-colorspace', 'Gray', '-negate',
                            '-bordercolor', 'white', '-border', '20', str(crop)],
                           env=env, check=True, capture_output=True)
            text = subprocess.check_output(['tesseract', str(crop), 'stdout', '-l', 'eng',
                                            '--psm', '7'], env=env, text=True,
                                           stderr=subprocess.PIPE).strip()
            results.append((scale, text))
    return results


def screenshot_wallet(path, env):
    reads = wallet_header_reads(path, env)
    values = set()
    for _, text in reads:
        match = re.fullmatch(r'LV(\d+)\s+KILLS\s*(\d{4,})', text, re.I)
        if match:
            values.add((int(match[1]), int(match[2])))
    assert len(values) == 1, f'Unreadable/ambiguous complete market wallet: {reads!r}'
    _, wallet = values.pop()
    return wallet, ' | '.join(f'{scale}%: {text}' for scale, text in reads)


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


def offer_identity(offer):
    return (normalized(offer['name']), offer['price'], normalized(offer['category']), offer['index'])


def market_header_text(path, env):
    text = crop_text(path, '150x24+612+209', env)
    assert text == 'BLACK MARKET', f'Current pixels do not show the market header: {text!r}'
    return text


def poll_market_offer(shot, env, index, prefix, deadline, observations, expected=None,
                      now=time.monotonic, pause=time.sleep):
    end = min(deadline, now() + 20)
    left = TILE_LEFTS[index]
    for attempt in range(10):
        assert now() < end, f'Offer {index} preview deadline exhausted: {observations}'
        image = shot(prefix + f'-inspect-{index}-{attempt}.png')
        header = market_header_text(image, env)  # Reject battle/unknown pixels before item reads.
        tile = tile_name_text(image, left, env)
        price = crop_text(image, f'39x23+{left+52}+384', env, digits=True)
        preview = crop_text(image, PREVIEW_CROP, env)
        record = dict(screenshot=image.name, header=header, tile=tile, price=price, preview=preview)
        observations.append(record)
        try:
            offer = inspected_offer(tile, price, preview, index)
            if expected is not None:
                assert offer_identity(offer) == offer_identity(expected), 'Selected offer identity changed'
        except AssertionError as error:
            record['error'] = str(error)
            pause(.2)
            continue
        assert now() < end, 'Offer verification exceeded remaining preview budget'
        return offer
    raise AssertionError(f'Offer {index} preview never matched exact tile/price/category: {observations}')


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
              and (match := re.fullmatch(r"EQUIPMENT\s*[·•.:+\"'-]?\s*(\d+)", text.strip(), re.I))}
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
                    'recruitahero', 'chooseanevolution', 'chooseabondtoreplace')) for text, *_ in rows)


def battle_level_pixels(path, env):
    """Read the tiny level/XP caption without alphabetic substitutes for digits.

    The numeric level and current XP may touch in OCR; their combined decimal
    payload still proves this exact HUD signature without inferring a level.
    """
    with tempfile.TemporaryDirectory(prefix='rogue-level-') as directory:
        crop = Path(directory) / 'level.png'
        subprocess.run(['convert', str(path), '-crop', '150x24+475+660',
                        '-resize', '300%', '-colorspace', 'Gray', '-negate',
                        '-bordercolor', 'white', '-border', '20', str(crop)],
                       env=env, check=True, capture_output=True)
        text = subprocess.check_output(['tesseract', str(crop), 'stdout', '-l', 'eng',
            '--psm', '7', '-c', 'tessedit_char_whitelist=LEVELXP0123456789/'],
            env=env, text=True, stderr=subprocess.PIPE).strip()
    assert re.fullmatch(r'LEVEL\s*\d+(?:\s+\d+)?\s*/\s*\d+\s*XP', text), \
        f'Unreadable numeric level/XP pixels: {text!r}'
    return text


def public_panel_state(rows, image=None, env=None):
    if any(find_text(rows, title) for title in ('NO LIVES REMAIN', 'CHAPTER CLEARED')):
        return 'terminal'
    if choice_pending(rows):
        return 'choice'
    if inventory_open(rows):
        return 'inventory'
    if has_gear_rows(rows):
        return 'market'
    if any(70 <= y < 650 and normalized(text) in
            ('attributes', 'legacytraining', 'expeditions', 'settings', 'paused', 'chooseanencounter')
            for text, _, y, *_ in rows):
        return 'unknown'
    if any(x < 100 and 70 <= y < 110 and re.match(r'^LIVES\s*\d+', text, re.I)
           or 450 <= x < 650 and 640 <= y < 695
           and re.fullmatch(r'LEVEL\s*\d+\s+\d+\s*/\s*\d+\s*XP', text.strip(), re.I)
           for text, x, y, *_ in rows):
        return 'battle'
    # A spatial LEVEL/numeric/XP candidate only requests fresh pixel validation;
    # missing separators or an S glyph never become guessed numeric values.
    if image is not None and any(450 <= x < 650 and 640 <= y < 695
            and re.fullmatch(r'LEVEL\s*(?:\d+|S)(?:\s+\d+)?(?:\s*/\s*\d+)?\s*X+P', text.strip(), re.I)
            for text, x, y, *_ in rows):
        battle_level_pixels(image, env)
        return 'battle'
    return 'unknown'


def wait_panel_open(shot, env, send, deadline, records, prefix, target_panel, visit=None):
    assert target_panel in ('inventory', 'market')
    requests, previous_clock = 0, None
    request_allowed = True
    while time.monotonic() < deadline:
        image = shot(prefix + f'-open-{len(records):02}.png')
        rows = ocr_screenshot(image, env)
        state = public_panel_state(rows, image, env)
        record = dict(visit=visit, screenshot=image.name, state=state, requests=requests)
        records.append(record)
        if state == target_panel:
            if state == 'inventory':
                inventory_count(rows)
            else:
                screenshot_wallet(image, env)
            return image, rows
        if state == 'terminal':
            return None  # Outer campaign loop owns the actual result/reward flow.
        assert time.monotonic() < deadline, 'Panel open observation exceeded visit budget'
        if state == 'choice':
            send('1')
            request_allowed = True  # Positive choice settlement permits a new battle epoch.
            record['input'] = '1'
            continue
        assert state == 'battle', f'Unexpected public screen while opening {target_panel}: {rows}'
        clock = screenshot_battle_seconds(image, env)
        assert read_toolbar_speed(image, env) == 1, 'Panel route requires actual1x'
        record['clock'] = clock
        assert time.monotonic() < deadline, 'Panel open OCR exceeded visit budget'
        # Advancing simulation time alone does not prove a queued toggle was
        # ignored. Never enqueue a second toggle within the same battle epoch.
        if request_allowed and (previous_clock is None or clock < previous_clock):
            assert requests < 3, f'{target_panel} failed to open after three public requests'
            send({'inventory': 'i', 'market': 'b'}[target_panel])
            requests += 1
            previous_clock = clock
            request_allowed = False
            record['input'] = {'inventory': 'i', 'market': 'b'}[target_panel]
    raise AssertionError(f'{target_panel} did not open within remaining visit budget')


class EquipmentPolicy:
    visit_limit = 12
    visit_budget_seconds = 120
    pause_budget_seconds = visit_limit * visit_budget_seconds

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
            equipment_visit_limit=self.visit_limit, equipment_visit_timeout_seconds=self.visit_budget_seconds,
            equipment_pause_budget_seconds=self.pause_budget_seconds,
            equipment_cooldown_after_completion_seconds=60)

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

    def inspect_offer(self, index, prefix, deadline):
        left = TILE_LEFTS[index]
        self.ui['call'](['xdotool', 'mousemove', '--window', self.ui['window'](),
                         str(left + 60), '365'])
        observations = []
        try:
            offer = poll_market_offer(self.ui['shot'], self.ui['env'], index, prefix,
                                      deadline, observations)
        except AssertionError:
            self.ui['report'].setdefault('equipment_hover_failures', []).append(
                dict(visit=self.visits, index=index, observations=observations))
            raise
        self.ui['report'].setdefault('equipment_hover_checks', []).append(
            dict(visit=self.visits, index=index, observations=observations,
                 stale_attempts=[row for row in observations if 'error' in row]))
        return offer

    def visible_countdown(self, prefix, deadline):
        while time.monotonic() < deadline:
            image = self.ui['shot'](prefix + '.png')
            try:
                return screenshot_battle_seconds(image, self.ui['env'])
            except AssertionError:
                # A level-up can open recruitment after the market closes.
                # Only a positively identified public choice permits a key;
                # unreadable ordinary HUDs still fail rather than being skipped.
                rows = self.ui['lines']()
                if not choice_pending(rows):
                    raise
                choice_image = self.ui['shot'](prefix + '-choice.png')
                self.ui['report'].setdefault('equipment_clock_choices', []).append(
                    dict(visit=self.visits, rows=rows, screenshot=choice_image.name))
                self.key('1')
        raise AssertionError('Choice queue did not expose a battle countdown within visit budget')

    def observe(self, rows, elapsed):
        if elapsed < self.next_visit or self.visits >= self.visit_limit:
            return
        self.visits += 1
        self.next_visit = elapsed + 60
        visit_started = time.monotonic()
        deadline = visit_started + self.visit_budget_seconds
        prefix = f'equipment-{self.visits:02}'
        with self.ui['input_lock']:
            clock = self.visible_countdown(prefix + '-unlock-clock', deadline)
            self.ui['report'].setdefault('equipment_unlock_checks', []).append(
                dict(visit=self.visits, countdown=clock, unlocked=clock < 22 * 60))
            if clock >= 22 * 60:
                self.ui['shot'](prefix + '-unavailable.png')
                return
            records = self.ui['report'].setdefault('equipment_open_attempts', [])
            opened = wait_panel_open(self.ui['shot'], self.ui['env'], self.key,
                                   deadline, records, prefix + '-market', 'market', self.visits)
            if opened is None:
                self.ui['report']['equipment_panel_terminal'] = records[-1]
                return
            market, rows = opened
            verified_market = market.with_name(prefix + '-market.png')
            shutil.copy2(market, verified_market)
            market = verified_market
            wallet, wallet_raw = screenshot_wallet(market, self.ui['env'])
            inspected = []
            for index, left in enumerate(TILE_LEFTS):
                assert time.monotonic() < deadline, 'Offer inspection exceeded visit budget'
                # Hover only: clicking a desktop tile buys immediately.
                inspected.append(self.inspect_offer(index, prefix, deadline))
            self.ui['report'].setdefault('equipment_inspections', []).append(
                dict(visit=self.visits, wallet=wallet, wallet_ocr=wallet_raw, offers=inspected))
            offers = affordable_gear(inspected, wallet)
            if offers:
                offer = offers[0]
                before = self.ui['shot'](prefix + '-before.png')
                hud, _, _ = read_hud_wallet(before, self.ui['env'])
                assert hud == wallet, ('Market/HUD wallet disagreement', hud, wallet)
                checked = self.inspect_offer(offer['index'], prefix + '-selected', deadline)
                assert offer_identity(checked) == offer_identity(offer), 'Selected offer changed before purchase'
                self.click(('', 1129, 482, 1239, 530))
                after = self.ui['shot'](prefix + '-after.png')
                remaining, _, raw = read_hud_wallet(after, self.ui['env'])
                market_remaining, market_raw = screenshot_wallet(after, self.ui['env'])
                assert remaining == market_remaining, ('Post-purchase Market/HUD disagreement', market_remaining, remaining)
                verify_deduction(wallet, remaining, offer)
                purchase = dict(name=offer['name'], price=offer['price'], wallet_before=wallet,
                                wallet_after=remaining, offer_ocr=offer, after_wallet_ocr=raw, after_market_ocr=market_raw,
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
                opened = wait_panel_open(self.ui['shot'], self.ui['env'], self.key,
                    deadline, records, prefix + '-inventory', 'inventory', self.visits)
                if opened is None:
                    self.ui['report']['equipment_panel_terminal'] = records[-1]
                    return
                _, inventory = opened
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
            assert time.monotonic() < deadline, 'Visit budget exhausted before battle-resume verification'
            self.ui['report'].setdefault('equipment_visit_timings', []).append(
                dict(visit=self.visits, phase='panels closed', elapsed=time.monotonic()-visit_started))
            before_seconds = self.visible_countdown(prefix + '-resume-before', deadline)
            # Modal recognition/settlement shares the remaining visit budget;
            # a separate ten-second window can expire before the next frame.
            resume_deadline = deadline
            while time.monotonic() < resume_deadline:
                time.sleep(.5)
                rows = self.ui['lines']()
                if choice_pending(rows):
                    self.key('1')
                    continue
                after_seconds = self.visible_countdown(prefix + '-resume-after', resume_deadline)
                if after_seconds < before_seconds:
                    self.ui['report'].setdefault('equipment_resume_proofs', []).append(
                        dict(visit=self.visits, before=before_seconds, after=after_seconds))
                    break
            else:
                raise AssertionError('Battle countdown did not resume after market/equipment closure')
            assert time.monotonic() < deadline, f'Market visit exceeded {self.visit_budget_seconds} seconds'
            self.ui['report']['equipment_visit_timings'].append(
                dict(visit=self.visits, phase='resume verified', elapsed=time.monotonic()-visit_started))
            # Leave a full minute of battle after a slow software-render visit.
            self.next_visit = elapsed + (time.monotonic()-visit_started) + 60
            print(json.dumps(dict(stage='earned equipment visit', visit=self.visits,
                                  purchases=len(self.purchases), wallet=wallet,
                                  visit_budget_seconds=self.visit_budget_seconds)), flush=True)

    def verify(self):
        assert self.purchases, 'No earned equipment purchased; active equipment route unproven'
        assert all(item.get('inventory_verified') for item in self.purchases)


if __name__ == '__main__':
    main(EquipmentPolicy)
