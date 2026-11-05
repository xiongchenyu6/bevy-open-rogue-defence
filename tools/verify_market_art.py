#!/usr/bin/env python3
"""Validate the real market catalog against its painted atlas; no image mutation.
Run: uv run --with pillow python tools/verify_market_art.py
"""
import hashlib
import json
from pathlib import Path
import re
from PIL import Image

root = Path(__file__).resolve().parents[1]
folder = root / 'assets/ui/market'
manifest = json.loads((folder / 'manifest.json').read_text())
source = (root / 'src/game/market.rs').read_text()
catalog = source.split('pub(crate) enum MarketItem {', 1)[1].split('}', 1)[0]
items = re.findall(r'^\s*(\w+),', catalog, re.M)
assert len(items) == len(manifest['items']) == 24
art_source = source.split('fn art_rect(self) -> Rect {', 1)[1].split('pub(crate) fn art_size', 1)[0]
indices = dict((name, int(index)) for name, index in re.findall(r'Self::(\w+) => (\d+),', art_source))
rect_source = art_source.split('const RECTS: [[f32; 4]; 24] = [', 1)[1].split('];', 1)[0]
rects = [[int(value) for value in re.findall(r'(\d+)\.', line)]
         for line in rect_source.splitlines() if '[' in line]
assert rects == [entry['rect'] for entry in manifest['items']], 'Runtime UVs differ from validated art regions'
assert indices == {item: index for index, item in enumerate(items)}, 'Runtime item art order differs'
image_path = folder / manifest['image']
assert hashlib.sha256(image_path.read_bytes()).hexdigest() == manifest['sha256']
image = Image.open(image_path)
assert image.mode == 'RGBA' and list(image.size) == manifest['size']
assert manifest['columns'] * manifest['rows'] == len(items)
alpha = image.getchannel('A')
for item, entry in zip(items, manifest['items']):
    assert item == entry['item'], (item, entry)
    index = entry['index']
    assert index == items.index(item)
    left, top, right, bottom = entry['rect']
    assert 0 <= left < right <= image.width and 0 <= top < bottom <= image.height
    bounds = alpha.crop((left, top, right, bottom)).point(
        lambda value: 255 if value > manifest['alpha_threshold'] else 0
    ).getbbox()
    assert bounds, item
    actual = [bounds[0] + left, bounds[1] + top, bounds[2] + left, bounds[3] + top]
    subject = entry['subject_bounds']
    assert all(abs(a - b) <= 1 for a, b in zip(actual, subject)), (item, actual)
    assert entry['rect'] == [subject[0] - 1, subject[1] - 1, subject[2] + 1, subject[3] + 1]
assert alpha.getextrema()[0] == 0, 'Atlas must retain transparent background'
print(json.dumps({'icons': len(items), 'size': image.size, 'alpha': True, 'catalog_matches': True}))
