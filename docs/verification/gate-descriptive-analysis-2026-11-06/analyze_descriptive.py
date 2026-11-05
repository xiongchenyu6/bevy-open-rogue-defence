from pathlib import Path
import hashlib,json,csv,collections,statistics,subprocess,shutil
ROOT=Path.cwd();INPUT=ROOT/'target/verification/gate-descriptive-inputs-2026-11-06';OUT=ROOT/'target/verification/gate-descriptive-analysis-2026-11-06';EXPECTED='75c69149c8d211daf9a4a5d37b0f1ee3755449fb70e79296dd665577ba0d95fc'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def write(n,v):(OUT/n).write_text(json.dumps(v,indent=2)+'\n')
def csvout(n,rows):
 fields=list(dict.fromkeys(k for row in rows for k in row))
 with (OUT/n).open('w',newline='') as f:
  w=csv.DictWriter(f,fieldnames=fields);w.writeheader();w.writerows({k:json.dumps(v) if isinstance(v,(list,dict)) else v for k,v in row.items()} for row in rows)
def parse(path):
 data=[]
 for line in path.read_text().splitlines():
  try:x=json.loads(line)
  except json.JSONDecodeError:continue
  if isinstance(x,dict):data.append(x)
 return data
binary=ROOT/'target/verification/sim-asset-root-r2-2026-11-06/balance-sim-snapshot';assert sha(binary)==EXPECTED
receipts={}
for name in ['asset-manifest.json','runner-inputs.json']:
 r=json.loads((INPUT/name).read_text())
 for x in r['files']:assert sha(INPUT/x['file'])==x['sha256'],x['file']
 receipts[name]=dict(files=len(r['files']),manifest_sha256=sha(INPUT/name),all_files_verified=True)
assert receipts['asset-manifest.json']['files']==185
runs={};case_rows=[];all_checkpoints=[];summaries={}
for rep in ['a','b']:
 folder=INPUT/'target/verification'/f'gate-descriptive-{rep}-2026-11-06';summary=json.loads((folder/'summary.json').read_text());summaries[rep]=summary
 assert summary['binary_sha256']==EXPECTED and summary['requested_cases']==len(summary['cases'])==24
 assert summary['jobs']==1 and summary['frames']==120000 and summary['pressure_override']==[] and summary['require_telemetry'] is True
 assert sha(folder/'simulator-snapshot')==EXPECTED
 assert json.loads((ROOT/f'target/verification/gate-descriptive-manager-2026-11-06/{rep}-process.json').read_text())['exit']==0
 for receipt in summary['cases']:
  name=receipt['name'];path=folder/(name+'.jsonl');data=parse(path);meta=next(x for x in data if x.get('type')=='metadata');provenance=next(x for x in data if x.get('type')=='runner_metadata');points=[x for x in data if x.get('type')=='checkpoint'];last=points[-1]
  assert receipt['valid'] and receipt['dimension_valid'] and receipt['telemetry_valid'] and receipt['returncode']==0 and not receipt['timed_out'] and not receipt['censored'],name
  assert last==receipt['last'] and len(points)==receipt['checkpoints'] and last['finished'] and receipt['outcome']==('win' if last['victory'] else 'loss'),name
  assert provenance['binary_sha256']==receipt['binary_sha256']==EXPECTED
  assert meta['executor']=='single-threaded' and meta['frame_cap']==120000 and meta['fixed_step_secs']==1/60 and meta['renderer'] is False
  assert points[0]['actual_hero_id']=={0:2,1:1,2:0}[meta['starter']]
  assert 'BALANCE_ASSET_ROOT '+str(INPUT/'assets') in path.read_text().splitlines()
  assert all(x['actual_chapter']==meta['chapter']==receipt['chapter'] and x['permanent_upgrades']==[0]*6 and x['damage_skill_casts'] is not None for x in points)
  assert all(x['skill_ranks']==sum(x['skill_ranks_by_id']) for x in points)
  assert len({x['frame'] for x in points})==len(points)
  first5=None
  for i,x in enumerate(points):
   if max(x['skill_ranks_by_id'])>=5:
    first5=dict(lower_elapsed=points[i-1]['elapsed'] if i else 0,first_observed_elapsed=x['elapsed'],first_observed_frame=x['frame']);break
  ranks=last['skill_ranks_by_id'];row=dict(replica=rep,case=name,chapter_number=meta['chapter']+1,policy='active' if name.startswith('active-') else 'passive',starter=meta['starter'],requested_seed=meta['seed'],actual_seed=points[0]['actual_run_seed'],initial_hero_id=points[0]['actual_hero_id'],final_hero_id=last['actual_hero_id'],outcome=receipt['outcome'],censored=False,exit=receipt['returncode'],checkpoints=len(points),final_wave=last['wave'],deaths=last['deaths'],elapsed=last['elapsed'],final_frame=last['frame'],level=last['level'],kills=last['kills'],attacks=last['attacks'],damage_skill_casts=last['damage_skill_casts'],gate_breaks=last['gate_breaks'],exposed_secs=last['exposed_secs'],learned_skill_count=sum(x>0 for x in ranks),skill_rank_sum=sum(ranks),skill_max_rank=max(ranks),rank_at_least5_count=sum(x>=5 for x in ranks),rank_histogram=dict(sorted(collections.Counter(ranks).items())),first_rank_at_least5_lower=first5['lower_elapsed'] if first5 else None,first_rank_at_least5_observed=first5['first_observed_elapsed'] if first5 else None,first_rank_at_least5_frame=first5['first_observed_frame'] if first5 else None)
  hero_path=[]
  for x in points:
   if not hero_path or hero_path[-1]!=x['actual_hero_id']:hero_path.append(x['actual_hero_id'])
  row.update(observed_hero_path=hero_path,observed_hero_switches=len(hero_path)-1)
  case_rows.append(row);runs[rep,name]=dict(meta=meta,points=points,row=row)
  for x in points:all_checkpoints.append(dict(replica=rep,case=name,**x))
assert {n for rep,n in runs if rep=='a'}=={n for rep,n in runs if rep=='b'}
pairs=[];diffs=[]
num=lambda x:isinstance(x,(float,int)) and not isinstance(x,bool)
for name in sorted(n for rep,n in runs if rep=='a'):
 a,b=runs['a',name],runs['b',name];assert a['meta']==b['meta']
 ta,tb=a['points'][-1],b['points'][-1];fields=sorted(k for k in ta.keys()|tb.keys() if ta.get(k)!=tb.get(k));fa={x['frame']:x for x in a['points']};fb={x['frame']:x for x in b['points']};common=sorted(fa.keys()&fb.keys());first=None;different=0
 for frame in common:
  pa,pb=fa[frame],fb[frame];changed={k:dict(a=pa.get(k),b=pb.get(k),delta_b_minus_a=pb[k]-pa[k] if num(pa.get(k)) and num(pb.get(k)) else None) for k in pa.keys()|pb.keys() if pa.get(k)!=pb.get(k)}
  if changed:
   different+=1
   if first is None:first=dict(frame=frame,elapsed_a=pa['elapsed'],elapsed_b=pb['elapsed'],fields=sorted(changed))
   for k,v in sorted(changed.items()):diffs.append(dict(case=name,frame=frame,metric=k,**v))
 pair=dict(case=name,chapter_number=a['row']['chapter_number'],policy=a['row']['policy'],starter=a['row']['starter'],requested_seed=a['row']['requested_seed'],outcome_a=a['row']['outcome'],outcome_b=b['row']['outcome'],outcome_equal=a['row']['outcome']==b['row']['outcome'],terminal_fields_equal=not fields,terminal_difference_fields=fields,common_checkpoint_count=len(common),different_common_checkpoint_count=different,unmatched_frames_a=sorted(fa.keys()-fb.keys()),unmatched_frames_b=sorted(fb.keys()-fa.keys()),first_common_difference=first)
 for metric in sorted(ta):
  if metric=='type':continue
  pair[metric+'_a']=ta[metric];pair[metric+'_b']=tb[metric]
  if num(ta[metric]) and num(tb[metric]):pair[metric+'_delta_b_minus_a']=tb[metric]-ta[metric]
 pairs.append(pair)
groups=[]
for key in sorted({(x['replica'],x['chapter_number'],x['policy']) for x in case_rows}):
 rows=[x for x in case_rows if (x['replica'],x['chapter_number'],x['policy'])==key];dist=lambda k:dict(sorted(collections.Counter(x[k] for x in rows).items()));observed=[x['first_rank_at_least5_observed'] for x in rows if x['first_rank_at_least5_observed'] is not None]
 groups.append(dict(replica=key[0],chapter_number=key[1],policy=key[2],cases=len(rows),outcome_counts=dist('outcome'),final_wave_counts=dist('final_wave'),deaths_counts=dist('deaths'),elapsed_min=min(x['elapsed'] for x in rows),elapsed_max=max(x['elapsed'] for x in rows),learned_count_min=min(x['learned_skill_count'] for x in rows),learned_count_max=max(x['learned_skill_count'] for x in rows),rank_sum_min=min(x['skill_rank_sum'] for x in rows),rank_sum_max=max(x['skill_rank_sum'] for x in rows),rank_at_least5_count_counts=dist('rank_at_least5_count'),first_at_least5_observed_cases=len(observed),first_at_least5_observed_min=min(observed) if observed else None,first_at_least5_observed_max=max(observed) if observed else None,final_hero_counts=dist('final_hero_id')))
report=dict(observed_hero_switch_cases=sum(x['observed_hero_switches']>0 for x in case_rows),active_observed_hero_switch_cases=sum(x['policy']=='active' and x['observed_hero_switches']>0 for x in case_rows),scope='48 descriptive observations, 24 matched A/B configurations, not independent population trials or exact internal replay',binary_sha256=EXPECTED,inputs=receipts,requested=48,valid=48,completed=48,censored=0,actual_nonzero_exits=0,matched_pairs=len(pairs),outcome_differences=sum(not x['outcome_equal'] for x in pairs),terminal_different_pairs=sum(not x['terminal_fields_equal'] for x in pairs),any_common_checkpoint_different_pairs=sum(x['different_common_checkpoint_count']>0 for x in pairs),different_checkpoint_metric_rows=len(diffs),groups=groups)
write('independent-review.json',report);write('paired-comparison.json',pairs);csvout('case-metrics.csv',case_rows);csvout('matched-ab-terminal.csv',pairs);csvout('checkpoint-differences.csv',diffs);csvout('checkpoint-curves.csv',all_checkpoints);csvout('groups.csv',groups)
print(json.dumps(report,indent=2))
