#!/usr/bin/env python3
"""Portable log/counter/source integrity; selected pixels are hash/size checked only."""
import ast
from collections import Counter
from decimal import Decimal
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
D=Path(__file__).resolve().parent
load=lambda name:json.loads((D/name).read_text())
sha=lambda data:hashlib.sha256(data).hexdigest()
manifest=load('SHA256.json')
actual_files={str(p.relative_to(D))for p in D.rglob('*')if p.is_file()and p.name!='SHA256.json'}
assert actual_files==set(manifest),('archive fileset differs',actual_files^set(manifest))
assert '.gitattributes' in manifest
for name,value in manifest.items():assert sha((D/name).read_bytes())==value,name
logs={}
for row in load('raw-log-manifest.json'):
    data=gzip.decompress((D/row['archive']).read_bytes())
    assert sha(data)==row['raw_sha256'] and len(data)==row['raw_bytes']
    name=row['archive'].removeprefix('logs/').removesuffix('.gz');logs[name]=data.decode()
A=re.compile(r'\x1b\[[0-9;]*m')
clean=lambda name:A.sub('',logs[name])
before=load('baseline-inputs.json');after=load('after-inputs.json')
assert set(before)==set(after) and len(before)==388
assert [k for k in before if before[k]!=after[k]]==['src/game/hit_feedback/enemy_flash.rs']
assert after==load('after-inputs-after.json')
assert sha((D/'source/enemy_flash-before.rs').read_bytes())==before['src/game/hit_feedback/enemy_flash.rs']
assert sha((D/'source/enemy_flash-after.rs').read_bytes())==after['src/game/hit_feedback/enemy_flash.rs']
for name in ('baseline','after','portrait'):
    text=(D/(name+'-unit-state.txt')).read_text()
    assert 'Result=success' in text and 'ExecMainStatus=0' in text and 'ActiveState=inactive' in text
for name in ('baseline-result.json','after-native-result.json','baseline-real-native-result.json','after-real-native-result.json','baseline-portrait-result.json','after-portrait-result.json','focused-tests-result.json','tier-tests-result.json','after-build-result.json','library-tests-result.json','clippy-result.json'):
    assert load(name)['returncode']==0,name
assert re.search(r'test result: ok\. 744 passed; 0 failed; 1 ignored;',clean('library-tests.log'))
assert 'error:' not in clean('clippy.log')
assert load('r2-emissive-alpha/tier-tests-result.json')['returncode']!=0
alpha=clean('r2-emissive-alpha/tier-tests.log')
assert 'alpha: 2.0' in alpha and 'alpha: 1.0' in alpha and 'test result: FAILED' in alpha
assert 'Intentionally stopped' in load('r1-format-scope/reason.json')['reason']
N=r'[-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?'
HEADER=re.compile(r'^(?:HIT_MATERIAL(?:_CONTACT|_BASELINE)?|ATTACK_TIMING|HIT_PUNCH_FIRST) frame=(\d+)\b')
FIELDS=('candidate_ordinal','progress_bits','base_damage','scaled_base_damage','raw_damage','final_damage','critical_roll','critical_roll_bits','crit_multiplier','hp_before','hp_after','hp_before_bits','hp_after_bits','hp_damage','rng_before_crit','rng_after_crit','rng_after_resolution','elapsed_bits')
def numeric(line,key):
    m=re.search(rf'(?<!\w){key}=({N})(?=\s|$)',line);assert m,(key,line)
    value=Decimal(m[1]);assert value.is_finite();return value

def parse(stage,portrait=False):
    filename=stage+('-portrait-native.log' if portrait else '-real-native.log')
    text=clean(filename)
    assert not re.search(r'ERROR|panicked|Validation Error|Caught rendering error',text)
    assert len(re.findall(r'\[capture\] wrote 250 simulation frames to ',text))==1
    saved=15 if portrait else 250
    assert re.search(rf'all {saved} screenshot\(s\) saved',text)
    assert 'HIT_MATERIAL_PASS hits=' in text and 'swapped=true restored=true source_and_control_unchanged=true' in text
    assert '[capture] PASS: real basic attack damage and weapon visuals;' in text
    contacts=[];timing=[];resolver=[];pending=[];crit=0
    for line in text.splitlines():
        if 'PROJECTILE_HIT' in line:
            profile=re.search(r'(?<!\w)profile=(.*?) base_damage=',line);assert profile
            values=tuple(numeric(line,key)for key in FIELDS)
            pending.append((values,profile[1]));crit+=numeric(line,'crit_multiplier')>1
        h=HEADER.match(line)
        if h and pending:
            resolver.extend((int(h[1]),*values,profile)for values,profile in pending);pending=[]
        if line.startswith('HIT_MATERIAL_CONTACT'):
            m=re.fullmatch(rf'HIT_MATERIAL_CONTACT frame=(\d+) actor=\S+ hp=({N}) damage=({N})',line);assert m
            contacts.append((int(m[1]),Decimal(m[2]),Decimal(m[3])))
        if line.startswith('ATTACK_TIMING'):
            m=re.fullmatch(rf'ATTACK_TIMING frame=(\d+) health=({N}) damage=({N}) clips=(.*)',line);assert m
            clips=ast.literal_eval(m[4]);assert all(isinstance(c,tuple)and len(c)==3 for c in clips)
            timing.append((int(m[1]),Decimal(m[2]),Decimal(m[3]),tuple((c[0],Decimal(str(c[1])),Decimal(str(c[2])))for c in clips)))
    assert not pending and len(contacts)==len(resolver)==60
    assert [x[0]for x in timing]==list(range(1,81))
    assert crit==1
    return tuple(map(Counter,(contacts,timing,resolver)))
for portrait in (False,True):
    a,b=parse('baseline',portrait),parse('after',portrait)
    assert all(x==y for x,y in zip(a,b)),('numeric contact/timing/resolver mismatch',portrait)
# These comparisons retain multiplicity/frame, all logged non-identity numeric
# resolver fields and DamageProfile. Only entity identities and outer ordinal
# are omitted; actor bindings/whole global determinism are not proven.
for stage in ('baseline','after'):
    text=clean(stage+'-native.log')
    assert 'wrote 180 simulation frames' in text and 'synthetic-hit-time' in text
    assert 'PASS dense-hit impact peak and exact production recovery' in text
    assert 'PASS ordinary contact does not inherit critical strength' in text
    assert not re.search(r'ERROR|panicked|Validation Error',text)
expected=set()
for stage in ('baseline','after'):
    for frame in (9,18,89,98,119,128):expected.add(f'frames/{stage}/frame{frame:05}.png')
    for suffix in ('real','portrait'):
        for frame in (19,102,109):expected.add(f'frames/{stage}-{suffix}/frame{frame:05}.png')
actual={str(p.relative_to(D))for p in (D/'frames').rglob('*.png')};assert actual==expected
for name in actual:
    data=(D/name).read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n'
    assert struct.unpack('>II',data[16:24])==((720,1280)if '-portrait/' in name else (1280,720))
comparison=load('comparison.json')
assert comparison['status']=='PASS' and comparison['analysis_revision']=='r2-color-space-contact-join'
assert all(comparison[k]for k in ('contact_multiset_equal','attack_timing_numeric_rows_equal','resolver_numeric_rows_equal'))
for stage in ('baseline','after'):
    report=comparison['material_matching'][stage]
    assert report['matched_contact_groups']==60 and not report['unmatched_contact_groups']
    assert report['contact_frame_material_groups']=={'ordinary':59,'critical':1}
    assert len(report['checked_material_rows'])==600 and not report['unclassified_fresh_swaps']
    assert comparison['runs'][stage]['sha256']==sha(gzip.decompress((D/('logs/'+stage+'-real-native.log.gz')).read_bytes()))
# r1 contains unresolved material classifications and is retained, not promoted.
assert load('comparison-r1.json')['status']=='PASS'
print(json.dumps(dict(result='PASS',frozen_input_files=388,changed_source=['enemy_flash.rs'],library=[744,0,1],desktop_contacts=60,portrait_contacts=60,timing_rows_per_run=80,resolver_rows_per_run=60,ordinary=59,critical=1,selected_pngs=24,limits='Selected PNG hashes/dimensions only; no portable claim checking every local frame, audio, full campaign, hardware or Steam'),indent=2))
