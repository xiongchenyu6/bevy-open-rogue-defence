#!/usr/bin/env python3
"""Mechanically trim, scale and pack generated item cutouts; never repaint art.

Run: uv run --with pillow python tools/build_recipe_art.py
Original generated files remain at the provenance paths. Runtime ships one PNG
because the existing engine does not enable WebP decoding.
"""
from pathlib import Path
import hashlib
import json
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
FOLDER = ROOT / 'assets/ui/recipes'
OUT = ROOT / 'docs/verification/recipe-art-2026-11-05'
CATALOG = ['Bloodreaver', 'AstralPrism', 'StormBastion', 'Suncoil', 'NightCrown', 'WorldEngine']


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    generation = json.loads((FOLDER / 'generation.json').read_text())
    atlas = Image.new('RGBA', (768, 512))
    records = []
    for index, (name, entry) in enumerate(zip(CATALOG, generation['sources'], strict=True)):
        source = Path(entry['path'])
        image = Image.open(source).convert('RGBA')
        bounds = image.getchannel('A').point(lambda value: 255 if value > 8 else 0).getbbox()
        assert bounds, name
        item = image.crop(bounds)
        item.thumbnail((224, 224), Image.Resampling.LANCZOS)
        x = (index % 3) * 256 + (256 - item.width) // 2
        y = (index // 3) * 256 + (256 - item.height) // 2
        atlas.alpha_composite(item, (x, y))
        records.append(dict(item=name, index=index,
            rect=[(index % 3) * 256, (index // 3) * 256,
                  (index % 3 + 1) * 256, (index // 3 + 1) * 256],
            source_size=list(image.size), source_sha256=hashlib.sha256(source.read_bytes()).hexdigest(),
            source_alpha_crop=list(bounds), packed_bounds=[x, y, x + item.width, y + item.height]))
    path = FOLDER / 'recipe-atlas.png'
    atlas.save(path, optimize=True)
    for record in records:
        left, top, _, _ = record['rect']
        bounds = atlas.crop(record['rect']).getchannel('A').point(
            lambda value: 255 if value > 8 else 0).getbbox()
        record['subject_bounds'] = [bounds[0] + left, bounds[1] + top,
                                    bounds[2] + left, bounds[3] + top]
    manifest = dict(image=path.name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        size=list(atlas.size), columns=3, rows=2, cell_size=256,
        packing='alpha>8 bbox trim only; proportional Lanczos fit224; centered in256cell; alpha retained',
        items=records)
    (FOLDER / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    sheet = Image.new('RGB', (900, 620), '#0b1114')
    draw = ImageDraw.Draw(sheet)
    for record in records:
        x = (record['index'] % 3) * 300 + 22
        y = (record['index'] // 3) * 310 + 12
        for iy in range(0, 256, 16):
            for ix in range(0, 256, 16):
                color = '#0b1114' if ((ix + iy) // 16) % 2 else '#294049'
                draw.rectangle((x + ix, y + iy, x + ix + 15, y + iy + 15), fill=color)
        sheet.paste(atlas.crop(record['rect']), (x, y), atlas.crop(record['rect']).getchannel('A'))
        draw.text((x, y + 266), record['item'], fill='#f3dfaa')
    sheet.save(OUT / 'contact-sheet.png', optimize=True)
    print(json.dumps(dict(icons=len(records), atlas=str(path.relative_to(ROOT)), bytes=path.stat().st_size)))


if __name__ == '__main__':
    main()
