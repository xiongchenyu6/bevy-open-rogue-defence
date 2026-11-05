#!/usr/bin/env python3
"""Public native pixels only; run with Tesseract/ImageMagick and tools/desktop on PYTHONPATH."""
import json
import sys
from pathlib import Path
from campaign_equipment_runtime import (screenshot_wallet, screenshot_battle_seconds,
    crop_text, selected_inventory_name, choice_pending)
from smoke_runtime import read_hud_wallet, normalized, ocr_screenshot

path = Path(sys.argv[1])
before, _ = screenshot_wallet(path/'before.png', None)
assert read_hud_wallet(path/'before.png')[0] == before
after = read_hud_wallet(path/'after.png')[0]
price = int(crop_text(path/'selected.png', '39x23+667+384', None, True))
assert before == 307 and price == 160 and before-after == price, (before, price, after)
name = selected_inventory_name(crop_text(path/'inventory.png', '840x25+220+445', None))
assert name == normalized('Wind Boots')
start = screenshot_battle_seconds(path/'resume-before.png', None)
end = screenshot_battle_seconds(path/'resume-after.png', None)
assert end < start, (start, end)
assert choice_pending(ocr_screenshot(path/'level10-recruitment.png'))
print(json.dumps(dict(before=before, after=after, price=price, item='Wind Boots',
    exact_deduction=True, selected_owned_detail=True, level10_choice_recognized=True,
    resumed_countdown_before=start, resumed_countdown_after=end), indent=2))
