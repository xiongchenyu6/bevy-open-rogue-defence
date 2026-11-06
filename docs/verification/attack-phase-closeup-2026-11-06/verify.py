#!/usr/bin/env python3
"""Portable broad/near diagnostic equality, not authored launch calibration."""
from collections import Counter
import gzip
import hashlib
import json
from pathlib import Path
import re
import struct
D=Path(__file__).resolve().parent
load=lambda n:json.loads((D/n).read_text())
sha=lambda b:hashlib.sha256(b).hexdigest()
manifest=load('SHA256.json')
assert set(manifest)=={str(p.relative_to(D))for p in D.rglob('*')if p.is_file()and p.name!='SHA256.json'}
assert '.gitattributes' in manifest
for n,h in manifest.items():assert sha((D/n).read_bytes())==h,n
logs={}
for row in load('raw-log-manifest.json'):
 data=gzip.decompress((D/row['archive']).read_bytes());assert len(data)==row['raw_bytes']and sha(data)==row['raw_sha256']
 logs[row['archive'].removeprefix('logs/').removesuffix('.gz')]=re.sub(r'\x1b\[[0-9;]*m','',data.decode())
a=load('baseline/inputs.json');b=load('inputs.json')
assert set(a)==set(b)and len(a)==390
assert [k for k in a if a[k]!=b[k]]==['src/bin/capture/attacks.rs']
assert b==load('inputs-after.json')
for stage,inputs in [('before',a),('after',b)]:assert sha((D/('source/attacks-'+stage+'.rs')).read_bytes())==inputs['src/bin/capture/attacks.rs']
old=(D/'source/attacks-before.rs').read_text();new=(D/'source/attacks-after.rs').read_text()
start=new.index('        if std::env::var_os("ROGUE_CAPTURE_RELEASE_TRACE").is_some() {')
end=new.index('        let mut cameras =',start)
assert new[:start]+new[end:]==old
assert 'remove::<rogue_commander::game::BattleCamera>()' in new[start:end]
assert 'if !growth {' in new[:start]
unit=(D/'unit-state.txt').read_text();assert 'Result=success' in unit and 'ExecMainStatus=0' in unit and 'ActiveState=inactive'in unit
for name in ('build','clippy','ember','storm','sword'):assert load(name+'-result.json')['returncode']==0
assert '-D' in load('clippy-result.json')['command'] and 'warnings' in load('clippy-result.json')['command']
assert 'error:' not in logs['clippy.log']
counts={'ember':24,'storm':13,'sword':0};result={}
for kind,wanted in counts.items():
 before=logs['baseline/current-'+kind+'.log'];after=logs[kind+'.log']
 for text in (before,after):
  assert not re.search(r'ERROR|panicked|Validation Error|Caught rendering error',text)
  assert '[capture] wrote 210 simulation frames to ' in text
  assert 'PASS: real basic attack damage' in text
 for receipt in (load('baseline/current-'+kind+'-result.json'),load(kind+'-result.json')):
  assert receipt['returncode']==0 and '210' in receipt['command']and 'attack-'+kind in receipt['command']
 summary={}
 for header,total in [('ATTACK_TIMING',80),('RELEASE_PHASE',210),('PROJECTILE_RELEASE',wanted)]:
  def extract(text):
   rows=[line for line in text.splitlines()if line.startswith(header+' ')]
   assert len(rows)==total,(kind,header,len(rows),total)
   assert all(re.search(r'\bframe=\d+\b',line)for line in rows)
   if header in ('ATTACK_TIMING','RELEASE_PHASE'):
    assert [int(re.search(r'\bframe=(\d+)',line)[1])for line in rows]==list(range(1,total+1))
   if header=='PROJECTILE_RELEASE':
    for line in rows:
     assert re.search(r'\bentity=\S+ added_tick=\d+ hero=\w+ position=Vec3\(.*\) velocity=Vec3\(.*\) clips=\[.*\]$',line)
    rows=[re.sub(r' entity=\S+ added_tick=\d+', '',line)for line in rows]
   # Retain frame, complete health/damage/position/velocity/clip/hero text,
   # duplicate row counts and all printed numeric precision.
   return Counter(rows)
  assert extract(before)==extract(after),(kind,header,'broad/near mismatch')
  report=load('comparison.json')[kind][header]
  assert report['count_before']==report['count_after']==total
  assert report['equal_excluding_projectile_identity_and_component_tick']is True
  summary[header]=total
 result[kind]=summary
expected={'frames/ember/frame00010.png','frames/ember/frame00013.png','frames/ember/frame00020.png','frames/ember/frame00023.png','frames/storm/frame00015.png','frames/storm/frame00019.png'}
assert expected=={str(p.relative_to(D))for p in (D/'frames').rglob('*.png')}
for name in expected:
 data=(D/name).read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n'and struct.unpack('>II',data[16:24])==(1280,720)
for model,hands in load('analysis/glb-hands.json').items():
 assert set(hands)=={'r','l'}
 for hand,record in hands.items():
  assert record['sha256']==a[record['file']]==b[record['file']]
  assert record['bone']=='handslot.'+hand and record['samples']
  assert record['source_clip']==('2H_Ranged_Shoot'if model=='ember_ranger'else'Spellcast_Shoot')
print(json.dumps(dict(result='PASS',source_delta='capture-only BattleCamera ownership guard',frozen_input_files=390,counters=result,selected_root_viewed_pngs=6,limits='No new timing fix, authored event, RNG equivalence, physical input/audio, fresh746 test rerun, campaign or hardware claim'),indent=2))
