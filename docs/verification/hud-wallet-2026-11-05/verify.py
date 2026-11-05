#!/usr/bin/env python3
"""Independently verify actual Arc Lens transaction pixels, not campaign completion."""
import json
import sys
from pathlib import Path
from smoke_runtime import read_hud_wallet, normalized
from campaign_equipment_runtime import screenshot_wallet, crop_text, inspected_offer, tile_name_crop

root = Path(sys.argv[1])
before = read_hud_wallet(root/'before.png')[0]
after = read_hud_wallet(root/'after.png')[0]
assert screenshot_wallet(root/'before.png', None)[0] == before == 291
assert screenshot_wallet(root/'after.png', None)[0] == after == 111
image = root/'before.png'
offer = inspected_offer(crop_text(image, tile_name_crop(1122), None),
    crop_text(image, '39x23+1174+384', None, True),
    crop_text(image, '425x19+695+489', None), 4)
assert normalized(offer['name']) == 'arclens' and offer['price'] == before-after == 180
notice = crop_text(root/'after.png', '640x24+615+563', None)
assert normalized(notice) == 'boughtarclens', notice
print(json.dumps(dict(before=before,after=after,price=offer['price'],
    matching_market_and_hud=True, actual_bought_notice=notice,
    selected_owned_inventory_detail_verified=False, completed_campaign=False),indent=2))
