from pathlib import Path
import subprocess,json,hashlib,shutil,os,time
r=Path.cwd();p=r/'target/verification/full-slot-variety-2026-11-07/native-before'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'inputs.json').write_text(json.dumps(a,indent=2))
for name in ['src/game/progression_ui.rs','src/game/progression_ui/layout.rs','src/game/progression.rs','src/game/progression/draft_touch_tests.rs','src/game/mod.rs','src/bin/capture.rs','src/bin/capture/phone_draft.rs','src/bin/capture/skill_variety.rs']:
 d=p/'source'/name;d.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(r/name,d)
def run(name,cmd,env=None,required=True):
 started=time.monotonic()
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-started}));assert inputs()==a,'frozen input changed';print(name,'TERMINAL',x.returncode,flush=True)
 if required:assert x.returncode==0,name

run('build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'capture';shutil.copyfile(r/'target/market-visual/debug/capture',exe);exe.chmod(0o755);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for size in ['1280x720','360x800']:
 for lang in ['en','zh']:
  name=f'{size}-{lang}';env=os.environ.copy();env.update(EXPECT_FULL_SLOT_VARIETY='0',ROGUE_COMMANDER_LANG=lang,ROGUE_CAPTURE_SAVE_FRAMES='100,159,179',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
  run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'180','skill-variety','0xC0126',size],env)
  assert 'PASS full-slot-variety before' in (p/(name+'.log')).read_text()
(p/'inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('FULL SLOT NATIVE BEFORE PASS',flush=True)
