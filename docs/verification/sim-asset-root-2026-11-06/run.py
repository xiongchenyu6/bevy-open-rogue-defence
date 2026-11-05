from pathlib import Path
import subprocess,json,hashlib,os,shutil,time
root=Path.cwd();out=root/'target/verification/sim-asset-root-r2-2026-11-06';inputs=root/'target/verification/gate-descriptive-inputs-2026-11-06'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,value):(out/name).write_text(json.dumps(value,indent=2)+'\n')
def run(args,name,timeout=900,env=None,expected=0):
    started=time.monotonic()
    with (out/(name+'.log')).open('w') as log:p=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,timeout=timeout,env=env)
    result={'args':args,'exit':p.returncode,'elapsed':time.monotonic()-started};write(name+'-process.json',result)
    print(name,result,flush=True)
    assert p.returncode==expected,(name,result)
def sources():
    files=sorted((root/'src').rglob('*.rs'))+[root/'Cargo.toml',root/'Cargo.lock']
    return {str(p.relative_to(root)):sha(p) for p in files}
run(['nix','develop','--command','rustfmt','--edition','2024','src/bin/balance_sim.rs'],'format',120)
before=sources();write('source-before.json',before)
run(['nix','develop','--command','cargo','test','--locked','--release','--bin','balance_sim','-j2'],'bin-tests')
run(['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'],'clippy')
run(['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'],'build')
after=sources();write('source-after.json',after);assert before==after,'Rust changed during verified build'
binary=out/'balance-sim-snapshot';shutil.copy2(root/'target/release/balance_sim',binary)
write('binary.json',{'path':str(binary),'sha256':sha(binary),'source_head':subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),'rust_diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','--binary','HEAD','--','src','Cargo.toml','Cargo.lock'])).hexdigest()})
for row in json.loads((inputs/'asset-manifest.json').read_text())['files']:assert sha(inputs/row['file'])==row['sha256'],row['file']
env=os.environ.copy();env.pop('ROGUE_SIM_TRACE_RANGE',None);env['RUST_LOG']='warn'
env.update(ROGUE_SIM_ASSET_ROOT=str(inputs/'assets'),ROGUE_SIM_CHAPTER='5',ROGUE_SIM_TRAINING='0',XDG_CONFIG_HOME=str(out/'smoke-state/config'),ROGUE_CAMPAIGN_SAVE=str(out/'smoke-state/campaign.save'),ROGUE_DETAIL_SAVE=str(out/'smoke-state/detail.save'))
(out/'smoke-state').mkdir()
run([str(binary),'passive','A11CE','2','behind','1200'],'snapshot-root-smoke',180,env)
rows=[]
for line in (out/'snapshot-root-smoke.log').read_text().splitlines():
    try:rows.append(json.loads(line))
    except json.JSONDecodeError:pass
assert ('BALANCE_ASSET_ROOT '+str(inputs/'assets')) in (out/'snapshot-root-smoke.log').read_text().splitlines()
assert any(r.get('type')=='checkpoint' for r in rows)
assert 'panicked at' not in (out/'snapshot-root-smoke.log').read_text()
invalid=env.copy();invalid['ROGUE_SIM_ASSET_ROOT']=str(out/'does-not-exist')
run([str(binary),'passive','A11CE','2','behind','1'],'missing-root-rejected',30,invalid,101)
assert 'simulation asset root is not a directory' in (out/'missing-root-rejected.log').read_text()
for row in json.loads((inputs/'asset-manifest.json').read_text())['files']:assert sha(inputs/row['file'])==row['sha256'],row['file']
write('result.json',{'passed':True,'scope':'Tool-only optional frozen asset root; existing bin tests, strict Clippy, build, actual finite startup with byte-identical185 assets and invalid-root rejection; not a full game outcome','asset_files':185,'asset_root':str(inputs/'assets'),'binary_sha256':sha(binary)})
print('SIM_ASSET_ROOT_FREEZE_PASS',flush=True)
