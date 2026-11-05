#!/usr/bin/env python3
"""Build after the prior Cargo owner finishes, then two gated replays only."""
import hashlib,json,pathlib,subprocess,time
ROOT=pathlib.Path.cwd()
OUT=ROOT/'target/verification/balance-collision-replay-2026-11-05'
PRIOR='rogue-inventory-split-r2-build-nov5.service'
PRIORLOG=ROOT/'target/verification/inventory-split-r2-2026-11-05/sequence.log'
def sha(data):return hashlib.sha256(data).hexdigest()
def receipt():
 files=[ROOT/'Cargo.toml',ROOT/'Cargo.lock']
 files+=list((ROOT/'src').rglob('*.rs'))+list((ROOT/'vendor/bevy_audio').rglob('*.rs'))
 diff=subprocess.check_output(['git','diff','--no-ext-diff','--binary','HEAD'])
 return dict(head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),
   status=subprocess.check_output(['git','status','--porcelain'],text=True),diff_sha256=sha(diff),
   source_sha256={str(p.relative_to(ROOT)):sha(p.read_bytes()) for p in sorted(files)})
def save(name,data):(OUT/name).write_text(json.dumps(data,indent=2)+'\n')
def run(name,args):
 print('START',name,flush=True)
 with (OUT/name).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert p.returncode==0,(name,p.returncode)
 print('PASS',name,flush=True)
print('WAIT prior Cargo owner',PRIOR,flush=True)
deadline=time.monotonic()+1800
while True:
 p=subprocess.run(['systemctl','--user','show',PRIOR,'-p','ActiveState','-p','Result'],capture_output=True,text=True,check=True)
 status=dict(line.split('=',1) for line in p.stdout.splitlines() if '=' in line)
 if status.get('ActiveState') not in ('active','activating','deactivating'):
  assert status.get('Result')=='success',status
  assert PRIORLOG.exists() and 'PASS capture-build.log' in PRIORLOG.read_text(),'prior sequence did not pass capture build'
  break
 assert time.monotonic()<deadline,'prior Cargo job wait timed out'
 time.sleep(5)
print('PRIOR TERMINAL PASS; acquiring single Cargo lane',flush=True)
before=receipt();save('source-before-build.json',before)
run('build.log',['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'])
after=receipt();save('source-after-build.json',after)
assert before['source_sha256']==after['source_sha256'],'source content changed during build; stop before replay'
snapshot=OUT/'balance-sim-snapshot'
import shutil
shutil.copy2(ROOT/'target/release/balance_sim',snapshot)
save('binary.json',dict(path=str(snapshot),sha256=sha(snapshot.read_bytes()),source_head=after['head'],source_diff_sha256=after['diff_sha256']))
common=['--frames','120000','--jobs','1','--case-timeout','900','--chapters','5','--training','0','--seeds','A11CE','--policies','active','--starters','2','--positions','behind','--require-telemetry']
for label in ('a','b'):
 run(f'replay-{label}.log',['python3','tools/run_balance_matrix.py','--binary',str(snapshot),'--batch',f'collision-repeat-{label}-nov5']+common)
a=ROOT/'target/verification/collision-repeat-a-nov5/active-h2-behind-sa11ce-c5-t0.jsonl'
b=ROOT/'target/verification/collision-repeat-b-nov5/active-h2-behind-sa11ce-c5-t0.jsonl'
p=subprocess.run(['python3','tools/compare_balance_replays.py',str(a),str(b)],capture_output=True,text=True)
(OUT/'comparison.json').write_text(p.stdout)
(OUT/'comparison.stderr').write_text(p.stderr)
def rows(path):return [r for line in path.read_text().splitlines() if line.startswith('{') and (r:=json.loads(line)).get('type')=='checkpoint']
x,y=rows(a),rows(b)
complete=bool(x and y and x[-1]['finished'] and y[-1]['finished'])
report=dict(complete=complete,all_checkpoint_fields_equal=x==y,left_count=len(x),right_count=len(y),strict_comparator_exit=p.returncode,matrix_started=False,scope='recorded checkpoints only; not proof of all internal state determinism')
for i,(u,v) in enumerate(zip(x,y)):
 if u!=v:
  report['first_difference']=dict(checkpoint=i,frame_left=u.get('frame'),frame_right=v.get('frame'),fields={k:[u.get(k),v.get(k)] for k in sorted(set(u)|set(v)) if u.get(k)!=v.get(k)})
  break
save('full-checkpoint-comparison.json',report)
print(json.dumps(report),flush=True)
print('TERMINAL: duplicate gate only; 24-case matrix remains withheld pending root review',flush=True)
