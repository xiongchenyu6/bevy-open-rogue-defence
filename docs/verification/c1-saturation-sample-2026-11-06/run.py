import pathlib,json,hashlib,subprocess,os,time,re,csv
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent;p=r/'target/verification/post-gate-cinder-repeat-2026-11-06';f=p/'frozen'
sha=lambda x:hashlib.sha256(x.read_bytes()).hexdigest()
state=dict(line.split('=',1) for line in subprocess.check_output(['systemctl','--user','show','rogue-post-order-repeat-nov6','-p','ActiveState','-p','Result','-p','ExecMainStatus','-p','InvocationID'],text=True).splitlines())
assert state['ActiveState']=='inactive' and state['Result']=='success' and state['ExecMainStatus']=='0',state
# Completed transient units may clear InvocationID; terminal state and all
# concrete four-stage receipts below are mandatory. Reject a different live ID.
assert state['InvocationID'] in ['', '1484cc5797b244b5ae6a77c595ac6640'],state
(o/'preflight-repeat-unit-state.json').write_text(json.dumps(state,indent=2)+'\n')
processes=json.loads((p/'manager-processes.json').read_text());assert len(processes)==4 and all(x['exit'] in [0,1] for x in processes)
assert processes[0]['exit']==processes[1]['exit']==processes[3]['exit']==0
binary=json.loads((p/'binary.json').read_text());pin=json.loads((p/'frozen-files.json').read_text())
for n,h in pin.items():assert sha(f/n)==h,n
assert sha(f/'balance-sim-snapshot')==binary['sha256']
source=json.loads((p/'inputs-after.json').read_text());assert all(sha(r/n)==h for n,h in source.items())
content=(r/'src/game/content.rs').read_text().split('pub const SKILLS:',1)[1].split('];',1)[0]
maxima=[int(n)for n in re.findall(r'max_rank:\s*(\d+)',content)];assert len(maxima)==32 and all(n>0 for n in maxima)
(o/'scope.json').write_text(json.dumps({'binary':binary,'max_ranks':maxima,'source_manifest':str(p/'inputs-after.json'),'case_count':8,'role':'descriptive fixed-policy sample, not causal tuning/global winrate; no UR branch/slot or earned progression claim','repeat_comparator_exit':processes[2]['exit']},indent=2)+'\n')
env={k:v for k,v in os.environ.items()if not k.startswith('ROGUE_')and k!='RUST_LOG'};env['ROGUE_SIM_ASSET_ROOT']=str(f/'assets')
args=['python3',str(f/'tools/run_balance_matrix.py'),'--binary',str(f/'balance-sim-snapshot'),'--batch','post-order-c1-sample-nov6','--frames','120000','--jobs','1','--case-timeout','900','--chapters','0','--training','0','--seeds','1,2,3,4','--policies','active,passive','--starters','2','--positions','behind','--require-telemetry']
t=time.monotonic()
with(o/'run.log').open('w')as log:q=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=7500)
(o/'process.json').write_text(json.dumps({'args':args,'exit':q.returncode,'seconds':time.monotonic()-t},indent=2)+'\n');assert q.returncode==0
batch=f/'target/verification/post-order-c1-sample-nov6';summary=json.loads((batch/'summary.json').read_text());assert len(summary['cases'])==8 and summary['binary_sha256']==binary['sha256']
rows=[];curves=[]
fields=['actual_run_seed','elapsed','wave','level','bosses','deaths','gate_breaks','exposed_secs','injured_secs','min_alive_health_fraction','max_health_deficit','attacks','damage_skill_casts','kills','gold','wood','actual_hero_id']
for case in summary['cases']:
 assert case['valid']and not case['timed_out']and case['returncode']==0,case
 stderr=pathlib.Path(case['stderr']).read_text();assert 'ERROR' not in stderr and 'panicked' not in stderr,case['name']
 assert 'BALANCE_ASSET_ROOT '+str(f/'assets') in pathlib.Path(case['log']).read_text()
 checkpoints=[]
 with pathlib.Path(case['log']).open()as log:
  for line in log:
   if line.startswith('{'):
    v=json.loads(line)
    if v.get('type')=='checkpoint':checkpoints.append(v)
 assert checkpoints and checkpoints[-1]==case['last']
 first=None;previous=0.
 for c in checkpoints:
  ranks=c['skill_ranks_by_id'];assert len(ranks)==len(maxima)
  learned=sum(n>0 for n in ranks);capped=sum(n>=m for n,m in zip(ranks,maxima))
  if capped and first is None:first=[previous,c['elapsed']]
  curves.append({'case':case['name'],'elapsed':c['elapsed'],'level':c['level'],'learned':learned,'capped':capped,'rank_total':sum(ranks),'bonds':len(c['owned_bond_ids'])})
  previous=c['elapsed']
 last=checkpoints[-1];row={'case':case['name'],'outcome':case['outcome'],**{key:last.get(key)for key in fields},'learned':learned,'capped':capped,'rank_total':sum(last['skill_ranks_by_id']),'bonds':len(last['owned_bond_ids']),'first_capped_observation_window':first};rows.append(row)
(o/'results.json').write_text(json.dumps(rows,indent=2)+'\n')
for name,data in [('cases',rows),('curves',curves)]:
 with(o/(name+'.csv')).open('w')as out:
  w=csv.DictWriter(out,fieldnames=list(data[0]));w.writeheader();w.writerows(data)
for n,h in pin.items():assert sha(f/n)==h,n
assert sha(f/'balance-sim-snapshot')==binary['sha256']
print(json.dumps({'cases':len(rows),'outcomes':{kind:sum(row['outcome']==kind for row in rows)for kind in sorted({row['outcome']for row in rows})},'scope':'descriptive fixed-policy sample only'}),flush=True)
