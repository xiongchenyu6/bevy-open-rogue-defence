#!/usr/bin/env python3
"""Read-only catalog/alpha/cell validation for the six generated recipe icons.

Run: uv run --with pillow python tools/verify_recipe_art.py
This validates art deliverables, not UI integration or native rendering.
"""
from pathlib import Path
import hashlib
import json
import re
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / 'assets/ui/recipes'
OUT = ROOT / 'docs/verification/recipe-art-2026-11-05'


def main():
    source = (ROOT / 'src/game/crafting.rs').read_text()
    enum = source.split('pub enum RecipeId {', 1)[1].split('}', 1)[0]
    catalog = re.findall(r'^\s*(\w+),', enum, re.M)
    manifest = json.loads((FOLDER / 'manifest.json').read_text())
    path = FOLDER / manifest['image']
    assert hashlib.sha256(path.read_bytes()).hexdigest() == manifest['sha256']
    image = Image.open(path)
    assert image.mode == 'RGBA' and list(image.size) == manifest['size'] == [768, 512]
    assert catalog == [entry['item'] for entry in manifest['items']]
    assert len(catalog) == manifest['columns'] * manifest['rows'] == 6
    rows = []
    hashes = []
    for index, entry in enumerate(manifest['items']):
        left, top = (index % 3) * 256, (index // 3) * 256
        assert entry['index'] == index and entry['rect'] == [left, top, left + 256, top + 256]
        cell = image.crop(entry['rect'])
        alpha = cell.getchannel('A')
        assert alpha.getextrema()[0] == 0 and alpha.getextrema()[1] > 200
        bounds = alpha.point(lambda value: 255 if value > 8 else 0).getbbox()
        assert bounds, entry['item']
        assert min(bounds[:2]) >= 16 and max(bounds[2:]) <= 240, (entry['item'], bounds)
        actual = [bounds[0] + left, bounds[1] + top, bounds[2] + left, bounds[3] + top]
        assert actual == entry['subject_bounds'], (entry['item'], actual)
        assert 0.05 < sum(alpha.histogram()[9:]) / (256 * 256) < 0.75
        # Transparent outer gutters cannot accidentally sample a neighboring icon.
        for rect in [(0, 0, 256, 16), (0, 240, 256, 256), (0, 0, 16, 256), (240, 0, 256, 256)]:
            assert alpha.crop(rect).getextrema() == (0, 0), (entry['item'], 'gutter')
        pixel_hash = hashlib.sha256(cell.tobytes()).hexdigest()
        hashes.append(pixel_hash)
        rows.append(dict(item=entry['item'], rect=entry['rect'], subject_bounds=actual,
                         transparent_gutter=16, cell_pixel_sha256=pixel_hash))
    assert len(set(hashes)) == 6, 'Recipes must have distinct art'
    report = dict(catalog_matches=True, icons=6, image=manifest['image'],
                  alpha=True, nonblank=True, distinct=True, cells_safe=True,
                  scope='Bitmap/catalog validation only; native UI acceptance is recorded separately',
                  items=rows)
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / 'verification.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
