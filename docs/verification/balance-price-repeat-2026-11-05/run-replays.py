import pathlib,subprocess,json,time
root=pathlib.Path.cwd();out=root/'target/verification/simulator-prices-2026-11-05'
def run(name,args):
 print('START',name,flush=True)
 with (out/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(name,p.returncode)
 print('PASS',name,flush=True)
while subprocess.run(['systemctl','--user','is-active','--quiet','rogue-sim-prices-after-nov5']).returncode==0: time.sleep(3)
assert 'PASS build.log' in (out/'verification-sequence.log').read_text(), 'Require actual verified new binary before replay'
common=['--frames','120000','--jobs','1','--case-timeout','900','--chapters','5','--training','0','--seeds','A11CE','--policies','active','--starters','2','--positions','behind','--require-telemetry']
for n in (1,2):
 binary='target/release/balance_sim' if n==1 else 'target/verification/balance-price-repeat-a-nov5/simulator-snapshot'
 run(f'replay-{n}.log',['python3','tools/run_balance_matrix.py','--binary',binary,'--batch',f'balance-price-repeat-{"a" if n==1 else "b"}-nov5']+common)
a=root/'target/verification/balance-price-repeat-a-nov5/active-h2-behind-sa11ce-c5-t0.jsonl';b=root/'target/verification/balance-price-repeat-b-nov5/active-h2-behind-sa11ce-c5-t0.jsonl'
p=subprocess.run(['python3','tools/compare_balance_replays.py',str(a),str(b)],capture_output=True,text=True);(out/'comparison.json').write_text(p.stdout)
def checkpoints(path):return [r for l in path.read_text().splitlines() if l.startswith('{') and (r:=json.loads(l)).get('type')=='checkpoint']
x,y=checkpoints(a),checkpoints(b);complete=bool(x and y and x[-1]['finished'] and y[-1]['finished']);report={'complete':complete,'all_checkpoint_fields_equal':x==y,'left_count':len(x),'right_count':len(y)}
for i,(u,v) in enumerate(zip(x,y)):
 if u!=v:report['first_difference']={'checkpoint':i,'fields':{k:[u.get(k),v.get(k)] for k in set(u)|set(v) if u.get(k)!=v.get(k)}};break
(out/'full-checkpoint-comparison.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps(report),flush=True)
if complete and x==y:
 run('matrix.log',['python3','tools/run_balance_matrix.py','--binary','target/verification/balance-price-repeat-a-nov5/simulator-snapshot','--batch','late-chapter-balance-2026-11-05','--frames','120000','--jobs','1','--case-timeout','900','--chapters','4,5','--training','0','--seeds','A11CE,BEEF','--policies','active,passive','--starters','0,1,2','--positions','behind','--require-telemetry'])
else:print('Replay divergence or truncation: matrix withheld; no numerical tuning justified',flush=True)
