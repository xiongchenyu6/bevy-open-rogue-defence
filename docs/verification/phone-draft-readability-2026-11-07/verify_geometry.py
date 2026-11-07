"""Recompute only geometry actually logged by the native phone probe."""
from pathlib import Path
import gzip,json,re

def vector(line, field):
    found=re.search(r'(?<![\w])'+re.escape(field)+r'=Vec2\(([-\d.eE]+), ([-\d.eE]+)\)',line)
    assert found,(field,line)
    return tuple(map(float,found.groups()))

def contains(center,size,child,child_size):
    return all(child[i]-child_size[i]*.5 >= center[i]-size[i]*.5-1.01
               and child[i]+child_size[i]*.5 <= center[i]+size[i]*.5+1.01 for i in [0,1])

def check(path,count,english,require_hint=False):
    raw=gzip.decompress(Path(path).read_bytes()).decode()
    titles={100:{},180:{}};layouts={};inners={};hints={}
    for line in raw.splitlines():
        if line.startswith('PHONE_TITLE '):
            frame=int(re.search(r'frame=(\d+)',line)[1])
            assert frame in titles
            text=json.loads(re.search(r'text=("(?:\\.|[^"\\])*")',line)[1])
            node=vector(line,'size');glyph=vector(line,'glyph_size')
            assert all(glyph[i] <= node[i]+1.01 for i in [0,1]),line
            if text=='Sharpshooter':
                font=float(re.search(r'font=Px\(([-\d.eE]+)\)',line)[1])
                assert glyph[1] <= font*1.3,line
            index=int(re.search(r'Build title (\d+)',line)[1])
            assert index not in titles[frame]
            titles[frame][index]=(text,vector(line,'position'),node)
        elif line.startswith('PHONE_GALLERY_HINT '):
            frame=int(re.search(r'frame=(\d+)',line)[1])
            assert frame in [100,180] and frame not in hints
            text=json.loads(re.search(r'text=("(?:\\.|[^"\\])*")',line)[1])
            size=vector(line,'size');glyph=vector(line,'glyph_size')
            assert all(glyph[i] <= size[i]+1.01 for i in [0,1])
            assert text==('6 cards · swipe to browse' if english else '共6张 · 上下拖动查看')
            hints[frame]=text
        elif line.startswith('PHONE_LAYOUT '):
            frame=int(re.search(r'frame=(\d+)',line)[1])
            assert int(re.search(r'count=(\d+)',line)[1])==count
            layouts[frame]=(vector(line,'last_card_center'),vector(line,'last_card_size'))
        elif line.startswith('PHONE_INNER_VISIBLE '):
            assert 'frame=180 ' in line
            part=re.search(r'part=(\w+)',line)[1]
            assert part not in inners
            inners[part]=(vector(line,'center'),vector(line,'size'))
    if require_hint:
        assert set(hints)==({100,180} if count>3 else set())
    assert set(layouts)=={100,180}
    assert all(set(rows)==set(range(count)) for rows in titles.values())
    assert set(inners)=={'title','art','pips'}
    for frame in [100,180]:
        assert contains(*layouts[frame],titles[frame][count-1][1],titles[frame][count-1][2])
    assert all(contains(*layouts[180],*child) for child in inners.values())
    if english and count==6:
        assert all(any(title[0]=='Sharpshooter' for title in rows.values()) for rows in titles.values())
    assert '[capture] PASS phone-draft: full catalog names shaped, drag did not choose, last card tap settled exactly once' in raw
    assert 'wrote 240 simulation frames' in raw
    assert 'panicked' not in raw and 'ERROR' not in raw
    return {'hint_observations':len(hints),'title_observations':sum(map(len,titles.values())),'last_card_inner_parts':sorted(inners),'logged_last_card_containment':True,'all_card_inner_containment':'executed native assertions in frozen probe; all other art/pips rectangles are not logged'}
