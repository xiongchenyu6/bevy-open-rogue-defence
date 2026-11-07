#!/usr/bin/env python3
"""Prepared portable archive verifier. Requires final R2 archive, receipts and hashes."""
from collections import Counter
import difflib
import gzip
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import sys

NUMBER=r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?'
f32=lambda value:struct.unpack('<f',struct.pack('<f',value))[0]
bits=lambda value:struct.unpack('<I',struct.pack('<f',value))[0]
from_bits=lambda value:struct.unpack('<f',struct.pack('<I',value))[0]

def numeric(line,key):
    found=re.search(rf'(?<!\w){key}=({NUMBER})(?=\s|$)',line)
    assert found,('missing actual resolver field',key)
    result=float(found[1]);assert math.isfinite(result)
    return result


def verify_producer_text(raw,modifiers):
    text=re.sub(r'\x1b\[[0-9;]*m','',raw)
    assert not re.search(r'ERROR|panicked at|Validation Error|Caught rendering error',text)
    assert text.count('[capture] wrote 480 simulation frames to ')==1
    assert re.search(r'all \d+ screenshot\(s\) saved after \d+ wait frame\(s\)',text)
    assert '[capture] PASS crown-detail:'in text
    assert f'modifier_ui={str(modifiers).lower()};'in text and '[capture] PASS crown-damage-preview:'in text
    cards=list(re.finditer(rf'CROWN_DAMAGE_CARD frame=(100|180) base_display=({NUMBER}) adjusted_display=(None|Some\(({NUMBER})\)) modifiers_expected=(true|false)',text))
    assert [int(row[1])for row in cards]==[100,180]
    for row in cards:
        assert format(float(row[2]),'.1f')=='650.8'
        assert row[5]==str(modifiers).lower()
        if modifiers:assert row[4]is not None and format(float(row[4]),'.1f')=='846.0'
        else:assert row[3]=='None'
    details=list(re.finditer(r'CROWN_DETAIL_TEXT frame=(100|180) language=\w+ fixed=true text=(.*)',text))
    assert [int(row[1])for row in details]==[100,180]
    for row in details:
        rendered=json.loads(row[2]);assert '6 → 6/6'in rendered and re.search(r'650\.8\s*→\s*650\.8',rendered)
        if modifiers:assert re.search(r'846\.0\s*→\s*846\.0',rendered)
    target=re.search(r'CROWN_DAMAGE_TARGET frame=200 actor=(\S+) .*initial_hp=50000 armor_component=false',text);assert target
    actor=target[1]
    witness=re.search(r'CROWN_DAMAGE_WYRM frame=(\d+) public_player_summon=true actual_scene4=true imported_mesh=true',text)
    assert witness and 200<int(witness[1])<=450
    intervals={}
    for row in re.finditer(rf'CROWN_DAMAGE_HP frame=(\d+) actor=(\S+) hp_before=({NUMBER}) hp_after=({NUMBER}) before_bits=(\d+) after_bits=(\d+)',text):
        if row[2]!=actor:continue
        pair=(int(row[5]),int(row[6]));assert pair not in intervals
        assert bits(float(row[3]))==pair[0]and bits(float(row[4]))==pair[1]
        intervals[pair]=int(row[1])
    expected={pair for pair,frame in intervals.items()if 200<frame<=450}
    assert expected
    matched=[];critical=[];tail=[];unmatched=[];seen=[]
    for line in text.splitlines():
        if 'PROJECTILE_HIT'not in line:continue
        enemy=re.search(r'(?<!\w)enemy=(\S+)',line)
        if not enemy or enemy[1]!=actor:continue
        assert re.search(r'element:\s*Frost\b',line)and re.search(r'class:\s*Summon\b',line)and re.search(r'skill_index:\s*Some\(26\)',line)
        pair=(int(numeric(line,'hp_before_bits')),int(numeric(line,'hp_after_bits')))
        item=dict(observed_frame=intervals.get(pair),hp_before_bits=pair[0],hp_after_bits=pair[1],base=f32(numeric(line,'base_damage')),hp_damage=numeric(line,'hp_damage'),critical_multiplier=numeric(line,'crit_multiplier'),raw=line)
        if pair not in intervals:unmatched.append(item);continue
        if intervals[pair]>450:tail.append(item);continue
        seen.append(pair)
        if item['critical_multiplier']!=1.:critical.append(item);continue
        outgoing=f32(item['base']*f32(1.30))
        assert format(item['base'],'.1f')=='650.8'and format(outgoing,'.1f')=='846.0'
        assert abs(item['hp_damage']-outgoing)<.001
        assert bits(f32(from_bits(pair[0])-outgoing))==pair[1]
        item['expected_crown_only']=outgoing;matched.append(item)
    # Coverage includes every observed interval, preserving multiplicity.
    assert Counter(seen)==Counter(expected),('missing/duplicate pre450 HP join',seen,expected)
    assert not unmatched and matched
    return dict(status='PASS'if len(matched)==7 and not critical else'REVIEW_REQUIRED_COUNT_OR_CRITICAL_CHANGE',matched_noncritical=matched,critical_before450=critical,unmatched_resolver=unmatched,excluded_post450=tail,pre450_intervals=len(expected),actor=actor)


def main():
    directory=Path(sys.argv[1]).resolve()
    load=lambda name:json.loads((directory/name).read_text())
    sha=lambda data:hashlib.sha256(data).hexdigest()
    manifest=load('SHA256.json')
    assert set(manifest)=={str(p.relative_to(directory))for p in directory.rglob('*')if p.is_file()and p.name!='SHA256.json'}
    assert '.gitattributes'in manifest
    for name,value in manifest.items():assert sha((directory/name).read_bytes())==value,name
    logs={}
    for row in load('raw-log-manifest.json'):
        data=gzip.decompress((directory/row['archive']).read_bytes())
        assert len(data)==row['raw_bytes']and sha(data)==row['raw_sha256']
        logs[row['archive'].removeprefix('logs/').removesuffix('.gz')]=data.decode()
    before,after=load('before-inputs.json'),load('after-inputs.json')
    assert after==load('after-inputs-after.json')
    provenance=load('source/provenance.json')
    for item in provenance['source_snapshots']:
        expected=(before if item['stage']=='before'else after)[item['path']]
        assert sha((directory/item['archive']).read_bytes())==expected
    actual_delta=sorted(k for k in set(before)|set(after)if before.get(k)!=after.get(k))
    assert actual_delta==provenance['changed_input_paths']
    tests=(directory/'source/after/src/game/skill_details/tests.rs').read_text()
    preview=(directory/'source/after/src/game/skill_details.rs').read_text()
    producer=(directory/'source/after/src/game/skills.rs').read_text()
    assert 'ember_rebirth_health' in preview and 'ember_rebirth_health' in producer
    assert 'rebirth_preview_uses_restored_health_before_its_actual_burst' in tests
    patch=[]
    for path in ('src/bin/capture.rs','src/bin/capture/crown_detail.rs'):
        original=(directory/'source/original-304a087'/path).read_text()
        installed=(directory/'source/before'/path).read_text()
        patch.extend(difflib.unified_diff(original.splitlines(keepends=True),installed.splitlines(keepends=True),fromfile='a/'+path,tofile='b/'+path))
    assert ''.join(patch)==(directory/'source/producer-fixture-installed.patch').read_text()
    for stage in ('before','after'):
        state=(directory/(stage+'-unit-state.txt')).read_text();assert 'Result=success'in state and 'ExecMainStatus=0'in state
    for name in ('before-build','focused-tests','library-tests','build','clippy'):
        assert load(name+'-result.json')['returncode']==0
    for test in ('adjusted_damage_uses_actual_crown_without_changing_the_base_formula','blood_wheel_previews_physical_outbound_and_flame_return_separately','rebirth_preview_uses_restored_health_before_its_actual_burst'):
        assert re.search(r'test .*'+test+r' \.\.\. ok',logs['focused-tests.log']),test
    assert re.search(r'test .*replacement_details_transfer_crown_bonus_without_mutating_the_live_build \.\.\. ok',logs['library-tests.log'])
    final_counts=load('final-test-counts.json')
    count_match=re.search(r'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;',logs['library-tests.log'])
    assert count_match and final_counts==dict(passed=int(count_match[1]),failed=0,ignored=int(count_match[2]))
    assert re.search(r'761 passed; 1 failed; 1 ignored;',logs['r1-missing-font-and-rebirth-review/library-tests.log'])
    assert re.search(r'763 passed; 0 failed; 1 ignored;',logs['r2-before-replacement-review/library-tests.log'])
    reports={}
    for stage in ('before','after'):
        for language in ('en','zh'):
            for portrait in (False,True):
                name=stage+('-portrait-'if portrait else'-')+language
                assert load(name+'-result.json')['returncode']==0 and load(name+'-verify-result.json')['returncode']==0
                reports[name]=verify_producer_text(logs[name+'.log'],stage=='after')
    assert len(reports)==8 and all(row['status']=='PASS'for row in reports.values()),reports
    images=load('root-viewed-images.json')
    assert len(images)==9 and len(set(images))==9
    assert {Path(name).parts[0]for name in images}==set(reports)
    assert {'frames/'+name for name in images}=={str(p.relative_to(directory))for p in (directory/'frames').rglob('*.png')}
    for name in images:
        data=(directory/'frames'/name).read_bytes()
        assert data[:8]==b'\x89PNG\r\n\x1a\n'
        assert struct.unpack('>II',data[16:24])==((720,1280)if 'portrait'in name else(1280,720))
    print(json.dumps(dict(status='PASS',final_library=final_counts,producer_cases=reports,scope='Final R3 archive only, actual typed producer/HP-bit/UI observations. Controlled fixture, sourceguard test evidence, no physical input or earned progression claim'),indent=2))

if __name__=='__main__':main()
