from pathlib import Path
import subprocess,json,hashlib,shutil,os,time
r=Path.cwd();p=r/'target/verification/full-slot-variety-2026-11-07/candidate-r9'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'inputs.json').write_text(json.dumps(a,indent=2))
for name in ['src/game/progression_ui.rs','src/game/progression_ui/layout.rs','src/game/progression.rs','src/game/progression/draft_touch_tests.rs','src/game/mod.rs','src/bin/capture.rs','src/bin/capture/phone_draft.rs','src/bin/capture/skill_variety.rs','src/game/progression/full_slot_tests.rs','src/game/progression_draft_tests.rs']:
 d=p/'source'/name;d.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(r/name,d)
def run(name,cmd,env=None,required=True):
 started=time.monotonic()
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-started}));assert inputs()==a,'frozen input changed';print(name,'TERMINAL',x.returncode,flush=True)
 if required:assert x.returncode==0,name

run('release-clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
run('focused',['nix','develop','--command','cargo','test','--locked','--release','--lib','full_slot_','-j2','--','--nocapture'])
run('build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'capture';shutil.copyfile(r/'target/market-visual/debug/capture',exe);exe.chmod(0o755);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
import re
seeds=dict(re.findall(r'FULL_SLOT_NATIVE_SEED rarity=(Sr|Ssr) seed=(\d+)',(p/'focused.log').read_text()))
for size in ['1280x720','360x800','320x667']:
 for lang in ['en','zh']:
  if size=='320x667' and lang=='zh':continue
  rarity='Sr' if (size=='1280x720')==(lang=='en') else 'Ssr'
  name=f'{size}-{lang}-{rarity}';env=os.environ.copy();env.update(EXPECT_FULL_SLOT_VARIETY='1',ROGUE_CAPTURE_VARIETY_SEED=format(int(seeds[rarity]),'x'),ROGUE_CAPTURE_VARIETY_RARITY=rarity,ROGUE_COMMANDER_LANG=lang,ROGUE_CAPTURE_SAVE_FRAMES='100,120,159,179',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
  run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'180','skill-variety','0xC0126',size],env,required=False)

for lang in ['en','zh']:
 name='long-320-'+lang;env=os.environ.copy();env.update(ROGUE_COMMANDER_LANG=lang,ROGUE_CAPTURE_SAVE_FRAMES='100,180,239',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
 run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'240','phone-draft-6','0xC0126','320x667'],env,required=False)


ordinary_seed=re.search(r'FULL_SLOT_ORDINARY_SEED seed=(\d+)',(p/'focused.log').read_text()).group(1)
for size in ['320x667','360x800']:
 name='ordinary-'+size;env=os.environ.copy();env.pop('ROGUE_CAPTURE_VARIETY_RARITY',None);env.update(EXPECT_FULL_SLOT_VARIETY='1',ROGUE_CAPTURE_VARIETY_ORDINARY='1',ROGUE_CAPTURE_VARIETY_SEED=format(int(ordinary_seed),'x'),ROGUE_COMMANDER_LANG='en',ROGUE_CAPTURE_SAVE_FRAMES='100,120,159,179',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
 run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'180','skill-variety','0xC0126',size],env,required=False)
run('library-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2'])
run('capture-clippy',['nix','develop','--command','cargo','clippy','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0','--','-D','warnings'])
run('bin-tests',['nix','develop','--command','cargo','test','--locked','--release','--bin','balance_sim','-j2'])
run('sim-build',['nix','develop','--command','cargo','build','--locked','--release','--bin','balance_sim','-j2'])
sim=p/'balance-sim-after';shutil.copyfile(r/'target/release/balance_sim',sim);sim.chmod(0o755);(p/'sim-binary.sha256').write_text(hashlib.sha256(sim.read_bytes()).hexdigest())
(p/'inputs-after.json').write_text(json.dumps(inputs(),indent=2))
failed=[f.name for f in p.glob('*-result.json') if json.loads(f.read_text())['returncode']!=0]
assert not failed,failed
print('FULL SLOT AFTER VERIFICATION PASS',flush=True)

frozen=r/'target/verification/full-slot-variety-2026-11-07/matched-inputs'
assert all(hashlib.sha256((frozen/k).read_bytes()).hexdigest()==v for k,v in a.items() if k.startswith('assets/'))
env={k:v for k,v in os.environ.items() if not k.startswith('ROGUE_') and k!='RUST_LOG'};env['ROGUE_SIM_ASSET_ROOT']=str(frozen/'assets')
for name,batch,policies,starters,seeds in [('after-matrix','full-slot-after-c1-2026-11-07','active,passive','0,1,2','1,2'),('after-repeat','full-slot-after-repeat-c1-2026-11-07','active','1','1')]:
 cmd=['nix','develop','--command','python3',str(frozen/'tools/run_balance_matrix.py'),'--binary',str(sim),'--batch',batch,'--frames','180000','--jobs','2' if name=='after-matrix' else '1','--case-timeout','900','--chapters','0','--training','0','--seeds',seeds,'--policies',policies,'--starters',starters,'--positions','behind','--require-telemetry']
 run(name,cmd,env)
print('FULL SLOT MATCHED AFTER12 PLUS REPEAT TERMINAL',flush=True)
