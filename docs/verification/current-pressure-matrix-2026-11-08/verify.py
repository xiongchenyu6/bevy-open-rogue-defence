import argparse,gzip,hashlib,itertools,json,re
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('archive',type=Path);p.add_argument('--local-root',type=Path);args=p.parse_args();D=args.archive.resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
summary=json.loads(raw['batch/summary.json']);assert len(summary['cases'])==24
assert json.loads(raw['manager/process.json'])['returncode']==0
binary='a6099bc9afc343c6840bbcb558182dfd8a543d2d248e7f7b83704302a1a5f2da';assert summary['binary_sha256']==binary
analysis=json.loads(raw['manager/analysis/analysis.json']);reported={x['case']:x for x in analysis['cases']};identities=set();groups={};caps=[]
for case in summary['cases']:
 match=re.fullmatch(r'(active|passive)-h([012])-behind-s(1|beef)-c([05])-t0',case['name']);assert match
 policy,starter,seed,chapter=match.groups();starter=int(starter);seed=int(seed,16);chapter=int(chapter);identity=(policy,starter,seed,chapter);assert identity not in identities;identities.add(identity)
 assert case['returncode']==0 and case['valid'] and case['dimension_valid'] and case['telemetry_valid'] and not case['censored'] and not case['timed_out'] and case['binary_sha256']==binary
 rows=[json.loads(l) for l in raw['batch/'+case['name']+'.jsonl'].decode().splitlines() if l.startswith('{')];cp=[x for x in rows if x.get('type')=='checkpoint'];assert len(cp)==case['checkpoints'] and cp[-1]==case['last'] and cp[-1]['finished']
 assert all(a['frame']<b['frame'] for a,b in zip(cp,cp[1:]))
 first=next(x for x in cp if x['elapsed']>0);assert first['actual_run_seed']==seed+1 and first['actual_hero_id']==[2,1,0][starter]
 assert all(x['actual_chapter']==chapter and x['permanent_upgrades']==[0]*6 for x in cp)
 last=cp[-1];ranks=last['skill_ranks_by_id'];assert len(ranks)==32 and all(isinstance(x,int) and 0<=x<=6 for x in ranks)
 record=reported[case['name']];assert record['outcome']==case['outcome']==('win' if last['victory'] else 'loss')
 assert record['rank_vector']==ranks and record['learned']==sum(x>0 for x in ranks) and record['stored_rank_total']==sum(ranks) and record['at_or_above_base_cap5']==sum(x>=5 for x in ranks)
 for k in ['elapsed','deaths','min_alive_health_fraction','gate_breaks','hero_hp','wave','level','kills','min_gate_hp','min_hero_hp','exposed_secs','injured_secs','max_health_deficit','max_hero_hp']:assert record[k]==last[k]
 previous=None;interval=None
 for x in cp:
  if any(v>=5 for v in x['skill_ranks_by_id']):interval=[previous,x['elapsed']];break
  previous=x['elapsed']
 assert record['first_base_cap_observation_interval']==interval
 if policy=='active' and interval:caps.append(interval)
 groups.setdefault((chapter,policy),[]).append(last)
assert identities==set(itertools.product(['active','passive'],range(3),[1,0xBEEF],[0,5]))
assert [(c,p,sum(x['victory'] for x in groups[(c,p)])) for c,p in [(0,'active'),(0,'passive'),(5,'active'),(5,'passive')]]==[(0,'active',6),(0,'passive',0),(5,'active',4),(5,'passive',0)]
assert len(caps)==4 and min(caps,key=lambda x:x[1])==[871.801,900.719]
active=[x for x in reported.values() if x['policy']=='active'];assert [min(x['learned'] for x in active),max(x['learned'] for x in active)]==[8,9]
assert [min(x['stored_rank_total'] for x in active),max(x['stored_rank_total'] for x in active)]==[9,20]
join=json.loads(raw['manager/source-join.json']);assert len(join['compiled_inputs'])==211 and join['frozen_pins']['balance-sim']==binary
assert sha((D/'policy/balance_sim.rs').read_bytes())==join['compiled_inputs']['src/bin/balance_sim.rs']
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
if args.local_root:
 for n,x in json.loads((D/'local-bulk-manifest.json').read_text()).items():
  f=args.local_root/n;assert f.stat().st_size==x['bytes'] and sha(f.read_bytes())==x['sha256'],n
 print('PASS read-only frozen binary/assets/runner hashes; no simulation')
print('PASS exact archive/raw bytes,all24 uncensored terminal identities,hero/seed mapping,outcomes,ranks and checkpoint pressure metrics.')
