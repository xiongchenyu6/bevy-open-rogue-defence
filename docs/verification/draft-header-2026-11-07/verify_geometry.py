"""Independent native rectangle and shaped text checker."""
import argparse
import itertools
import json
import re
from pathlib import Path

N = r'-?\d+(?:\.\d+)?(?:e[+-]?\d+)?'
RECT = re.compile(r'DRAFT_LAYOUT frame=(\d+) name=("[^"]+") min=Vec2\((' + N + r'), (' + N + r')\) max=Vec2\((' + N + r'), (' + N + r')\)')
TEXT = re.compile(r'DRAFT_TEXT frame=(\d+) node_size=Vec2\((' + N + r'), (' + N + r')\) glyph_size=Vec2\((' + N + r'), (' + N + r')\) glyph_count=(\d+)')
BUTTONS = ['Selection detail toggle', 'Build refresh', 'Build abandon', 'Build later']

def contains(a, b):
    return b[0] >= a[0]-1 and b[1] >= a[1]-1 and b[2] <= a[2]+1 and b[3] <= a[3]+1

def overlaps(a, b):
    return a[0] < b[2]-.5 and a[2] > b[0]+.5 and a[1] < b[3]-.5 and a[3] > b[1]+.5

def inspect(path, width, height):
    frames, texts = {}, {}
    raw = path.read_text()
    for m in RECT.finditer(raw):
        rows = frames.setdefault(int(m[1]), {})
        name = json.loads(m[2])
        assert name not in rows
        rows[name] = tuple(float(m[i]) for i in range(3, 7))
    for m in TEXT.finditer(raw):
        frame = int(m[1])
        assert frame not in texts
        v = [float(m[i]) for i in range(2, 6)]
        assert int(m[6]) > 0 and v[2] <= v[0]+1 and v[3] <= v[1]+1
        texts[frame] = v
    assert set(frames) == set(texts) == {100, 180}
    for frame, rows in frames.items():
        cards = sorted(n for n in rows if n.startswith('Build card '))
        assert len(cards) == 3
        assert set(rows) == set(['Draft header', 'Draft heading', 'Draft actions'] + BUTTONS + cards + ['Settings open', 'Detail viewport'])
        for name, r in rows.items():
            assert r[2] > r[0] and r[3] > r[1] and contains((0,0,width,height), r), (frame,name,r)
        assert contains(rows['Draft header'], rows['Draft heading']) and contains(rows['Draft header'], rows['Draft actions'])
        for name in BUTTONS:
            r = rows[name]
            assert min(r[2]-r[0],r[3]-r[1]) >= 43.5
            assert contains(rows['Draft actions'],r)
        for name in ['Draft heading', 'Detail viewport'] + BUTTONS + cards:
            assert not overlaps(rows['Settings open'], rows[name]), (frame, 'Settings open', name)
        for a,b in itertools.combinations(['Draft heading'] + BUTTONS + cards,2):
            assert not overlaps(rows[a],rows[b]), (frame,a,b)
    assert raw.count('PASS draft-layout frame=') == 2
    return {'rectangles': frames, 'shaped_heading': texts}

if __name__ == '__main__':
    p=argparse.ArgumentParser()
    p.add_argument('log',type=Path)
    p.add_argument('width',type=int)
    p.add_argument('height',type=int)
    a=p.parse_args()
    print(json.dumps(inspect(a.log,a.width,a.height),indent=2))
