from pathlib import Path
import json,hashlib
p=Path(__file__).resolve().parent
manifest=json.loads((p/'manifest.json').read_text())
assert set(manifest)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='manifest.json'}
for name,row in manifest.items():
 data=(p/name).read_bytes();assert len(data)==row['bytes']and hashlib.sha256(data).hexdigest()==row['sha256'],name
scope=json.loads((p/'scope.json').read_text());maxima=scope['max_ranks'];assert len(maxima)==32 and set(maxima)=={5}
s=json.loads((p/'summary.json').read_text());results={r['case']:r for r in json.loads((p/'results.json').read_text())};assert len(s['cases'])==len(results)==8
for case in s['cases']:
 assert case['valid']and not case['timed_out']and not case['censored']and case['returncode']==0
 records=[]
 for line in (p/'raw'/Path(case['log']).name).read_text().splitlines():
  if line.startswith('{'):
   r=json.loads(line)
   if r.get('type')=='checkpoint':records.append(r)
 assert records[-1]==case['last']and records[-1]['finished']
 first=None;previous=0.
 for c in records:
  ranks=c['skill_ranks_by_id'];assert len(ranks)==32
  if first is None and any(n>=m for n,m in zip(ranks,maxima)):first=[previous,c['elapsed']]
  previous=c['elapsed']
 last=records[-1];row=results[case['name']];ranks=last['skill_ranks_by_id']
 assert row['first_capped_observation_window']==first
 assert row['learned']==sum(n>0 for n in ranks)and row['capped']==sum(n>=m for n,m in zip(ranks,maxima))and row['rank_total']==sum(ranks)
 assert row['outcome']==case['outcome']and row['elapsed']==last['elapsed']
 assert last['actual_chapter']==0 and last['permanent_upgrades']==[0]*6
 assert case['binary_sha256']==scope['binary']['sha256']
active=[r for n,r in results.items()if n.startswith('active-')];passive=[r for n,r in results.items()if n.startswith('passive-')]
assert len(active)==len(passive)==4
assert all(r['outcome']=='win'and r['deaths']==0 and 8<=r['learned']<=9 and 1<=r['capped']<=2 for r in active)
assert all(r['outcome']=='loss'and r['deaths']==3 and r['capped']==0 for r in passive)
assert json.loads((p/'process.json').read_text())['exit']==0
print('Archived 8-case descriptive sample PASS; no global balance/determinism claim')
