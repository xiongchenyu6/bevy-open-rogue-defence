"""Bounded public-market refresh policy; pixel evidence only, no game state."""
import re
import subprocess
import time
from campaign_equipment_runtime import (crop_text, market_header_text, screenshot_wallet,
    tile_name_text, TILE_LEFTS)
from smoke_runtime import normalized, read_hud_wallet

# Actual 1280x720 English market arrow, sampled from archived f5 market-18.png.
# This verifies the visible button marker before its rendered fee is accepted.
MARKER = '000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000011111110000000000000000110000011100000000000001100000000110000000000011000000000011100000000010000000000011111000000110000000000001111000000110000000000001110000000110000000000001100000000110000000000001000000000010000000000000000000000011000000000000000000000001100000000000000000000000110000000000000000000000011111000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000'


def refresh_control(image, env):
    market_header_text(image, env)
    pixels = subprocess.check_output(['convert', str(image), '-crop', '24x24+898+603',
        '+repage', '-colorspace', 'Gray', '-threshold', '40%', '-depth', '8', 'gray:-'], env=env)
    assert len(pixels) == len(MARKER), 'Refresh marker outside supported public panel'
    differences = sum((value > 0) != (expected == '1') for value, expected in zip(pixels, MARKER))
    assert differences <= 12, 'Visible refresh arrow marker not recognized'
    text = crop_text(image, '60x28+944+602', env, digits=True)
    assert re.fullmatch(r'[1-9]\d*', text), 'Visible refresh fee unreadable'
    return dict(fee=int(text), center=(910, 615), fee_ocr=text, marker_differences=differences)


def refresh_allowed(wallet, fee, visits, visit, count, spent):
    return (isinstance(fee, int) and fee > 0 and visit not in visits and count < 6
            and spent + fee <= 360 and wallet >= fee + 300)


def refresh_observation(image, env, before, fee, old_names):
    market_header_text(image, env)
    wallet, raw = screenshot_wallet(image, env)
    hud, _, hud_raw = read_hud_wallet(image, env)
    assert hud == wallet, 'Refresh market/HUD wallet disagreement'
    record = dict(screenshot=image.name, wallet=wallet, hud=hud, wallet_ocr=raw, hud_ocr=hud_raw)
    if wallet == before:
        return record
    assert before - wallet == fee, 'Refresh wrong debit'
    names = tuple(normalized(tile_name_text(image, left, env)) for left in TILE_LEFTS)
    assert all(names), 'Post-refresh stock names unreadable'
    changed = names != tuple(old_names)
    # Market presentation HelpText: 20px feedback row between preview and
    # refresh button, calibrated against actual f5 1280EN market-18 pixels.
    notice = False if changed else normalized(crop_text(image, '630x24+614+562', env)) == 'stockrefreshed'
    record.update(exact_debit=True, stock_refreshed_notice=notice, stock_names=names,
                  stock_changed=changed)
    return record


def observe_one_refresh(click, shot, read, wallet, fee, deadline, observations, prefix,
                        now=time.monotonic, pause=time.sleep):
    """Exactly one click; no corrective input while awaiting real passive proof."""
    end = min(deadline, now() + 20)
    assert now() < end, 'Refresh observation deadline expired before input'
    observations.append(dict(input='refresh_click', wallet_before=wallet, fee=fee, wall=now()))
    click()
    for attempt in range(20):
        assert now() < end, 'Refresh observation deadline exhausted'
        image = shot(f'{prefix}-refresh-{attempt:02}.png')
        record = read(image)
        observations.append(record)
        assert now() < end, 'Refresh observation exceeded deadline'
        if record.get('exact_debit'):
            assert wallet - record['wallet'] == fee and record['hud'] == record['wallet'], 'Refresh wrong debit'
            if record.get('stock_refreshed_notice') or record.get('stock_changed'):
                return image, record['wallet']
        else:
            assert record['wallet'] == wallet and record['hud'] == wallet, 'Unproved refresh wallet change'
        pause(.2)
    raise AssertionError('Refresh debit and stock evidence not observed; never reclick')
