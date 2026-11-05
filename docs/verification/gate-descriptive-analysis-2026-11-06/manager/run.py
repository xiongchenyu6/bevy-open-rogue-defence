from pathlib import Path
import os,sys,json,hashlib,subprocess,time
root=Path.cwd();out=root/'target/verification/gate-descriptive-manager-2026-11-06';inputs=root/'target/verification/gate-descriptive-inputs-2026-11-06'
binary=root/'target/verification/sim-asset-root-r2-2026-11-06/balance-sim-snapshot';expected='75c69149c8d211daf9a4a5d37b0f1ee3755449fb70e79296dd665577ba0d95fc'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(out/name).write_text(json.dumps(value,indent=2)+'\n')
def check_inputs():
    assert sha(binary)==expected
    for name in ('asset-manifest.json','runner-inputs.json'):
        for row in json.loads((inputs/name).read_text())['files']:assert sha(inputs/row['file'])==row['sha256'],row['file']
check_inputs()
source=json.loads((root/'target/verification/sim-asset-root-r2-2026-11-06/result.json').read_text());assert source['passed'] and source['binary_sha256']==expected
scope={'scope':'Descriptive repeated balance observations only; exact numeric repeat gate remains open. No pressure overrides, tuning, real save-unlock or rendered-player acceptance claim.','gameplay_commit':subprocess.check_output(['git','rev-parse','237e650'],text=True).strip(),'simulator_commit':subprocess.check_output(['git','rev-parse','5f35790'],text=True).strip(),'binary_sha256':expected,'frozen_inputs':str(inputs),'batches':2,'cases_per_batch':24,'frames':120000,'jobs':1,'case_timeout':900,'chapter_indices':[4,5],'training':0,'seeds':['A11CE','BEEF'],'policies':['active','passive'],'starters':[0,1,2],'positions':['behind'],'profile':'Simulator prior-chapters-cleared and zero six-upgrade fixture; not a normally earned checkpoint','trace':'Full actor/contact tracing disabled; actual checkpoints and damage-skill telemetry retained','limitations':['Two seeds and two repetitions per configuration are descriptive, not population win rates.','Passive still accepts mandatory recruitment; active chooses first cards, absorbs and attempts purchases.','Censored/frame-cap/invalid cases must remain explicit; never count them as completed wins or losses.']}
write('scope.json',scope)
env={k:v for k,v in os.environ.items() if not k.startswith('ROGUE_') and k!='RUST_LOG'}
env.update(ROGUE_SIM_ASSET_ROOT=str(inputs/'assets'),RUST_LOG='warn',RUST_BACKTRACE='1')
for replica in ('a','b'):
    check_inputs();batch='gate-descriptive-'+replica+'-2026-11-06'
    args=[sys.executable,str(inputs/'tools/run_balance_matrix.py'),'--binary',str(binary),'--batch',batch,'--frames','120000','--jobs','1','--case-timeout','900','--chapters','4,5','--training','0','--seeds','A11CE,BEEF','--policies','active,passive','--starters','0,1,2','--positions','behind','--require-telemetry']
    write(replica+'-command.json',args);started=time.monotonic();print('START descriptive replica',replica,flush=True)
    with (out/(replica+'.log')).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,env=env,timeout=21900)
    write(replica+'-process.json',{'exit':p.returncode,'elapsed':time.monotonic()-started})
    summary_path=inputs/'target/verification'/batch/'summary.json'
    assert p.returncode==0,'Invalid batch retained; no retry or following replica'
    summary=json.loads(summary_path.read_text());assert summary['requested_cases']==len(summary['cases'])==24 and summary['binary_sha256']==expected
    for case in summary['cases']:
        assert case['valid'],case['name']
        log_path=summary_path.parent/(case['name']+'.jsonl')
        assert 'BALANCE_ASSET_ROOT '+str(inputs/'assets') in log_path.read_text().splitlines()
    check_inputs();print('COMPLETE descriptive replica',replica,'censored',sum(c['censored'] for c in summary['cases']),flush=True)
write('result.json',{'execution_complete':True,'requested_cases':48,'replicas':['a','b'],'scope':'Completed descriptive sampling execution only; inspect raw outcomes/censoring before interpretation. Exact numeric determinism and release quality remain unproven.'})
print('DESCRIPTIVE_SAMPLING_EXECUTION_COMPLETE',flush=True)
