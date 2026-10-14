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
from campaign_runtime import GAME, main
from smoke_runtime import find_text, normalized, read_hud_wallet


def market_wallet(rows):
    values = {int(match[1]) for text, x, y, *_ in rows
              if x > 900 and 240 < y < 290
              and (match := re.search(r'KILLS\s*(\d+)\b', text, re.I))}
    if not values:
        return None
    assert len(values) == 1, f'Ambiguous market wallet: {values}'
    return values.pop()


def affordable_gear(rows, wallet):
    offers = []
    for row in rows:
        text, x, y, *_ = row
        match = re.search(r'\[[^\]]*\bGEAR\]\s*(.*?)\s+(\d+)\s*KILLS\b', text, re.I)
        if match and x > 900 and 290 < y < 600:
            price = int(match[2])
            if 0 < price <= wallet:
                offers.append(dict(name=match[1].strip(), price=price, row=row))
    # Fixed cheapest-first policy, not a claimed optimized human strategy.
    return sorted(offers, key=lambda offer: (offer['price'], offer['name']))


def inventory_count(rows):
    counts = {int(match[1]) for text, x, y, *_ in rows
              if 200 <= x < 600 and 80 <= y < 145
              and (match := re.fullmatch(r'EQUIPMENT\s*[·•.:-]?\s*(\d+)', text.strip(), re.I))}
    assert len(counts) == 1, f'Actual equipment header count missing/ambiguous: {counts}'
    return counts.pop()


def last_owned_icon(count):
    # Fixed verified 1280x720 Gear grid: 10 columns; first two rows visible.
    # Fail truthfully instead of guessing a clipped icon when inventory grows.
    assert 1 <= count <= 20, f'Inventory count {count} exceeds verified visible grid bound 1..20'
    index = count - 1
    return 245 + 86 * (index % 10), 184 + 86 * (index // 10)


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
        self.ui['call'](['xdotool', 'key', key])
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
                if any(any(token in normalized(text) for token in
                           ('forgeanewbond', 'chooseyour', 'shapeyourarsenal', 'recruitahero'))
                       for text, *_ in rows):
                    self.key('1')
                else:
                    break
                assert time.monotonic() < deadline, 'Existing card queue exceeded market visit budget'
            self.key('b')
            rows = self.ui['lines']()
            wallet = market_wallet(rows)
            if wallet is None:
                assert not find_text(rows, 'BLACK MARKET'), 'Open market wallet could not be read'
                # Locked market does not toggle open; retain actual UI for review.
                self.ui['shot'](prefix + '-unavailable.png')
                return
            self.ui['shot'](prefix + '-market.png')
            offers = affordable_gear(rows, wallet)
            if offers:
                offer = offers[0]
                before = self.ui['shot'](prefix + '-before.png')
                hud, _, _ = read_hud_wallet(before, self.ui['env'])
                assert hud == wallet, ('Market/HUD wallet disagreement', hud, wallet)
                self.click(offer['row'])
                after = self.ui['shot'](prefix + '-after.png')
                remaining, _, raw = read_hud_wallet(after, self.ui['env'])
                assert wallet - remaining == offer['price'], ('Wrong purchase deduction', wallet, remaining, offer)
                purchase = dict(name=offer['name'], price=offer['price'], wallet_before=wallet,
                                wallet_after=remaining, offer_ocr=offer['row'], after_wallet_ocr=raw,
                                visit=self.visits, before=before.name, after=after.name)
                self.purchases.append(purchase)
            self.key('b')
            assert market_wallet(self.ui['lines']()) is None, 'Market failed to close'
            if offers:
                self.key('i')
                inventory = self.ui['lines']()
                count = inventory_count(inventory)
                x, y = last_owned_icon(count)
                self.click(('', x, y, x, y))
                purchase['inventory_count'] = count
                purchase['selected_icon'] = [x, y]
                while time.monotonic() < deadline:
                    inventory = self.ui['lines']()
                    if any(normalized(offer['name']) in normalized(text) for text, *_ in inventory):
                        break
                    time.sleep(.3)
                else:
                    raise AssertionError('Purchased item not visible in actual inventory')
                self.ui['shot'](prefix + '-inventory.png')
                close = find_text(inventory, 'Close')
                assert close, 'Actual equipment Close button not visible'
                self.click(close)
                assert not find_text(self.ui['lines'](), 'EQUIPMENT'), 'Equipment did not close'
                purchase['inventory_verified'] = True
            assert time.monotonic() < deadline, 'Market visit exceeded 45 seconds'
            print(json.dumps(dict(stage='earned equipment visit', visit=self.visits,
                                  purchases=len(self.purchases), wallet=wallet)), flush=True)

    def verify(self):
        assert self.purchases, 'No earned equipment purchased; active equipment route unproven'
        assert all(item.get('inventory_verified') for item in self.purchases)


if __name__ == '__main__':
    main(EquipmentPolicy)
