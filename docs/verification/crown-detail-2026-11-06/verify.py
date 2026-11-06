#!/usr/bin/env python3
"""Portable rank6 preview regression + unchanged base-rank text verification."""
import ast
from collections import Counter
from decimal import Decimal
import gzip
import hashlib
import itertools
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
 raw=gzip.decompress((D/row['archive']).read_bytes());assert len(raw)==row['raw_bytes'] and sha(raw)==row['raw_sha256']
 logs[Path(row['archive']).name.removesuffix('.gz')]=re.sub(r'\x1b\[[0-9;]*m','',raw.decode())
a,b=load('before-inputs.json'),load('after-inputs.json')
assert set(a)==set(b) and len(a)==390
assert [k for k in a if a[k]!=b[k]]==['src/game/skill_details.rs']
assert b==load('after-inputs-after.json')
for stage,inputs in [('before',a),('after',b)]:assert sha((D/(stage+'-skill_details.rs')).read_bytes())==inputs['src/game/skill_details.rs']
shared=load('source/provenance.json')['shared_capture_and_test_sources']
for n,h in shared.items():assert sha((D/'source'/n).read_bytes())==a[n]==b[n]==h
before=(D/'before-skill_details.rs').read_text();after=(D/'after-skill_details.rs').read_text()
next_expr='current.max(current.saturating_add(1).min(skill.definition().max_rank))'
cap_expr='current.max(skill.definition().max_rank)'
assert after.count(next_expr)==2 and after.count(cap_expr)==2
restored=after.replace(next_expr,'current.saturating_add(1).min(skill.definition().max_rank)').replace(cap_expr,'skill.definition().max_rank')
assert restored==before,'Production UI delta must be exactly four expressions'
for stage in ('before','after','portrait'):
 text=(D/(stage+'-unit-state.txt')).read_text();assert 'Result=success' in text and 'ExecMainStatus=0' in text and 'ActiveState=inactive' in text
assert load('before-regression-result.json')['returncode']==101
assert 'test result: FAILED. 0 passed; 1 failed;' in logs['before-regression.log']
assert '6 → 5/5' in logs['before-regression.log']
for name in ('before-base','before-build','after-regression','after-base','after-build','library-tests','clippy'):
 assert load(name+'-result.json')['returncode']==0,name
assert 'test result: ok. 746 passed; 0 failed; 1 ignored;' in logs['library-tests.log']
assert 'error:' not in logs['clippy.log']
base={}
for stage in ('before','after'):
 rows={}
 for line in logs[stage+'-base.log'].splitlines():
  if not line.startswith('BASE_RANK_DETAIL '):continue
  m=re.fullmatch(r'BASE_RANK_DETAIL index=(\d+) language=(en|zh) rank=(\d+) texts=(.*)',line);assert m
  identity=(int(m[1]),m[2],int(m[3]));assert identity not in rows
  texts=ast.literal_eval(m[4]);assert isinstance(texts,tuple)and len(texts)==2 and all(isinstance(v,str)for v in texts)
  rows[identity]=texts
 assert set(rows)==set(itertools.product(range(32),('en','zh'),range(6)))
 base[stage]=rows
assert base['before']==base['after']
fixture=(D/'source/src/bin/capture/crown_detail.rs').read_text()
assert 'build.skills[SKILL] = 5;' in fixture and 'build.skills[SKILL] = 6;' not in fixture
assert 'probe.wallet - probe.price' in fixture and 'real purchase exact kill debit' in fixture
assert 'real G draw must create selection UI' in fixture and 'paths, [0, 1, 2]' in fixture
report={}
for stage,lang,portrait in itertools.product(('before','after'),('en','zh'),(False,True)):
 key=stage+('-portrait-' if portrait else '-')+lang;text=logs[key+'-native.log'];receipt=load(key+'-native-result.json')
 assert receipt['returncode']==0
 assert '240' in receipt['command'] and 'crown-detail' in receipt['command'] and ('720x1280' if portrait else '1280x900') in receipt['command']
 assert not re.search(r'ERROR|panicked|Validation Error',text)
 assert len(re.findall(r'\[capture\] wrote 240 simulation frames to ',text))==1
 assert ('all 2 screenshot(s) saved' if portrait else 'all 32 screenshot(s) saved') in text
 assert 'CROWN_DETAIL_PURCHASE frame=50 actual_rank=6' in text
 assert 'PASS crown-detail: actual Crown purchase, exact kill debit, rank5-to6, real G UR branches and actual localized detail;' in text
 details={};damage={}
 for line in text.splitlines():
  if line.startswith('CROWN_DETAIL_TEXT '):
   m=re.fullmatch(r'CROWN_DETAIL_TEXT frame=(\d+) language=(English|SimplifiedChinese) fixed=(true|false) text=(.*)',line);assert m
   assert m[2]==('English'if lang=='en'else'SimplifiedChinese') and m[3]==('true'if stage=='after'else'false')
   rendered=json.loads(m[4]);wanted=('Rank 'if lang=='en'else'等级 ')+('6 → 6/6'if stage=='after'else'6 → 5/5')
   assert wanted in rendered
   details[int(m[1])]=rendered
  if line.startswith('CROWN_DETAIL_DAMAGE '):
   m=re.fullmatch(r'CROWN_DETAIL_DAMAGE frame=(\d+) current=([\d.]+) next=([\d.]+) actual_rank=6 branches=3',line);assert m
   assert Decimal(m[2])==Decimal('650.8') and Decimal(m[3])==Decimal('650.8'if stage=='after'else'573.1')
   damage[int(m[1])]=[m[2],m[3]]
 assert set(details)==set(damage)=={100,180}
 report[key]=damage
expected={f'frames/{stage}{"-portrait" if portrait else ""}-{lang}/frame{frame:05}.png'for stage,lang,portrait,frame in itertools.product(('before','after'),('en','zh'),(False,True),(99,179))}
assert expected=={str(p.relative_to(D))for p in (D/'frames').rglob('*.png')}
for n in expected:
 data=(D/n).read_bytes();assert data[:8]==b'\x89PNG\r\n\x1a\n' and struct.unpack('>II',data[16:24])==((720,1280)if '-portrait-' in n else (1280,900))
print(json.dumps(dict(result='PASS',frozen_files=390,production_delta_expressions=4,base_observations=384,strings_per_observation=2,before_regression_exit=101,native_cases=8,frames_per_case=240,selected_pngs=16,damage=report,library=[746,0,1],limit='Rank/header downgrade only; final outgoing Crown30% buff inclusion remains unverified'),indent=2))
