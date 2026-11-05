from pathlib import Path
import csv,hashlib,json,itertools,sys
p=Path(__file__).resolve().parent;read=lambda n:json.loads((p/n).read_text());sha=lambda x:hashlib.sha256(x.read_bytes()).hexdigest();binary='75c69149c8d211daf9a4a5d37b0f1ee3755449fb70e79296dd665577ba0d95fc'
for name,digest in read('evidence-hashes.json').items():assert sha(p/name)==digest,name
assert len(list((p/'raw').rglob('*.jsonl')))==len(list((p/'raw').rglob('*.stderr')))==48
runs={};counts={'win':0,'loss':0};points_total=0
for replica in ['a','b']:
 summary=read(f'raw/{replica}/summary.json');assert summary['requested_cases']==len(summary['cases'])==24 and summary['binary_sha256']==binary and summary['pressure_override']==[]
 assert read(f'manager/{replica}-process.json')['exit']==0
 dimensions=set()
 for case in summary['cases']:
  assert case['returncode']==0 and not case['timed_out'] and case['valid'] and case['dimension_valid'] and case['telemetry_valid'] and not case['censored']
  records=[]
  for line in (p/'raw'/replica/(case['name']+'.jsonl')).read_text().splitlines():
   if line.startswith('{'):records.append(json.loads(line))
  checkpoints=[v for v in records if v.get('type')=='checkpoint'];metadata=next(v for v in records if v.get('type')=='metadata');provenance=next(v for v in records if v.get('type')=='runner_metadata')
  assert provenance['binary_sha256']==binary and checkpoints[-1]==case['last'] and len(checkpoints)==case['checkpoints']
  assert checkpoints[-1]['finished'] and metadata['renderer'] is False and metadata['executor']=='single-threaded' and metadata['frame_cap']==120000
  assert all(v['permanent_upgrades']==[0]*6 and len(v['skill_ranks_by_id'])==32 and v['skill_ranks']==sum(v['skill_ranks_by_id']) and v['damage_skill_casts'] is not None for v in checkpoints)
  outcome='win' if checkpoints[-1]['victory'] else 'loss';assert case['outcome']==outcome;counts[outcome]+=1;points_total+=len(checkpoints)
  dimensions.add((metadata['chapter'],metadata['starter'],metadata['seed'],metadata['policy']))
  runs[replica,case['name']]=checkpoints
 expected=set(itertools.product([4,5],[0,1,2],[int('a11ce',16),int('beef',16)],['greedy-card-zero','passive']));assert dimensions==expected
outcome_diff=terminal_diff=checkpoint_diff=field_rows=0
for name in sorted(n for rep,n in runs if rep=='a'):
 a,b=runs['a',name],runs['b',name];outcome_diff+=a[-1]['victory']!=b[-1]['victory'];terminal_diff+=a[-1]!=b[-1]
 aa={x['frame']:x for x in a};bb={x['frame']:x for x in b};different=False
 for frame in aa.keys()&bb.keys():
  fields=[k for k in aa[frame].keys()|bb[frame].keys() if aa[frame].get(k)!=bb[frame].get(k)];different|=bool(fields);field_rows+=len(fields)
 checkpoint_diff+=different
assert (outcome_diff,terminal_diff,checkpoint_diff,field_rows)==(1,15,14,5884)
assert counts=={'win':21,'loss':27} and points_total==1440
assert len(list(csv.DictReader((p/'matched-ab-terminal.csv').open())))==24
assert len(list(csv.DictReader((p/'checkpoint-differences.csv').open())))==5884
assert len(list(csv.DictReader((p/'checkpoint-curves.csv').open())))==1440
if len(sys.argv)>1:
 root=Path(sys.argv[1])
 for manifest in ['asset-manifest.json','runner-inputs.json']:
  rows=read('frozen-inputs/'+manifest)['files']
  for row in rows:assert sha(root/row['file'])==row['sha256'],row['file']
 for row in read('binary-references.json').values():assert sha(p.parents[2]/row['path'])==binary
 command=read('manager/a-command.json');assert sha(Path(command[command.index('--binary')+1]))==binary
print('PASS: 48 real finished/exit0/uncensored, 24 pairs with1 outcome/15 terminal/14 commoncheckpoint differences, 1440points; descriptive not exact replay/population winrate')
