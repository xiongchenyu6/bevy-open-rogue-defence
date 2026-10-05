"""Check that every catalog choice has a valid shipped icon, without Blender."""

import json
import re
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CONTENT = (ROOT / 'src/game/content.rs').read_text()
ICONS = ROOT / 'assets/ui/icons'


def catalog(section, end):
    return re.findall(r'id: "([a-z_]+)"', CONTENT.split(section)[1].split(end)[0])


def check_png(path, expected):
    data = path.read_bytes()
    assert data[:8] == b'\x89PNG\r\n\x1a\n', f'invalid PNG: {path}'
    offset = 8
    payload = bytearray()
    while offset < len(data):
        length = struct.unpack_from('>I', data, offset)[0]
        kind = data[offset + 4:offset + 8]
        chunk = data[offset + 8:offset + 8 + length]
        checksum = struct.unpack_from('>I', data, offset + 8 + length)[0]
        assert zlib.crc32(kind + chunk) == checksum, f'corrupt PNG: {path}'
        if kind == b'IHDR':
            assert struct.unpack_from('>II', chunk) == expected, f'wrong size: {path}'
            assert chunk[8:10] == bytes([8, 6]), f'expected 8-bit RGBA: {path}'
        if kind == b'IDAT':
            payload.extend(chunk)
        offset += length + 12
    assert len(zlib.decompress(payload)) == expected[1] * (expected[0] * 4 + 1), path


skills = catalog('pub const SKILLS:', 'pub const ENEMIES:')
reference = (ROOT / 'src/game/bond_catalog.rs').read_text()
expanded = (ROOT / 'src/game/bond_catalog/expanded.rs').read_text()
card_source = reference.split('const BASE_CARDS:')[1].split('const BASE_FAMILIES:')[0] + expanded.split('const CARDS:')[1].split('const FAMILIES:')[0]
family_source = reference.split('const BASE_FAMILIES:')[1].split('const fn joined')[0] + expanded.split('const FAMILIES:')[1]
def icon_mapping(source):
    return dict(re.findall(r'id:\s*"([a-z_]+)".*?icon:\s*"([a-z_]+)"', source, re.S))
cards = icon_mapping(card_source)
families = icon_mapping(family_source)
manifest = json.loads((ICONS / 'manifest.json').read_text())
assert skills == manifest['skills'], 'stale skill icons'
assert cards == manifest['bond_cards'], 'stale card icons'
assert families == manifest['bond_families'], 'stale family icons'
bonds = manifest['bonds']
icons = skills + bonds + manifest['hud']
assert len(icons) == len(set(icons)), 'duplicate icon IDs'
for name in set(icons + list(cards.values()) + list(families.values())):
    check_png(ICONS / f'{name}.png', (256, 256))
heroes = catalog('pub const HEROES:', 'pub const SKILLS:')
for name in heroes:
    check_png(ROOT / f'assets/ui/portraits/hero_{name}.png', (384, 480))
print(f'PASS: {len(skills)} skill icons, {len(bonds)} bond icons, '
      f'{len(manifest["hud"])} action icons, {len(heroes)} portraits; '
      f'{len(cards)} cards / {len(families)} families mapped; PNG checksums and payloads valid')
