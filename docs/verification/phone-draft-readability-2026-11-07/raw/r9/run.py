from pathlib import Path
import subprocess,json,hashlib,shutil,os,time
r=Path.cwd();p=r/'target/verification/phone-draft-readability-2026-11-07/r9'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'inputs.json').write_text(json.dumps(a,indent=2))
for name in ['src/game/progression_ui.rs','src/game/progression_ui/layout.rs','src/game/progression.rs','src/game/progression/draft_touch_tests.rs','src/game/mod.rs','src/bin/capture.rs','src/bin/capture/phone_draft.rs']:
 d=p/'source'/name;d.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(r/name,d)
shutil.copyfile(r/'docs/verification/current-package-2026-11-07/frames/smoke/public-portrait-skill.png',p/'before-public-360.png')
def run(name,cmd,env=None,required=True):
 started=time.monotonic()
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode,'elapsed':time.monotonic()-started}));assert inputs()==a,'frozen input changed';print(name,'TERMINAL',x.returncode,flush=True)
 if required:assert x.returncode==0,name
for name,cmd in [('touch-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','scroll_card_','-j2']),('layout-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','phone_gallery_','-j2']),('library-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2']),('clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings']),('build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])]:run(name,cmd)
run('capture-clippy',['nix','develop','--command','cargo','clippy','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0','--','-D','warnings'])
exe=p/'capture';shutil.copyfile(r/'target/market-visual/debug/capture',exe);exe.chmod(0o755);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for size in ['320x667','360x800']:
 for lang in ['en','zh']:
  for count in [3,6]:
   name=f'after-{size}-{lang}-{count}';env=os.environ.copy();env.update(ROGUE_COMMANDER_LANG=lang,ROGUE_CAPTURE_SAVE_FRAMES='100,180,205,239',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
   run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'240',f'phone-draft-{count}','0xC0126',size],env)
   assert 'PASS phone-draft:' in (p/(name+'.log')).read_text()
for name,size,scenario,frames in [('supp-desktop','1280x720','crown-detail',240),('supp-compact','844x390','crown-detail',240),('supp-continuous-desktop','1280x720','immediate-skill',320),('supp-build-skill','1280x720','build-skill',180),('supp-build-bond','1280x720','build-bond',180)]:
 env=os.environ.copy();env.update(EXPECT_CROWN_DETAIL_FIXED='1',ROGUE_CAPTURE_DRAFT_LAYOUT='1',ROGUE_COMMANDER_LANG='en',ROGUE_CAPTURE_SAVE_FRAMES='100,180,239',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
 run(name,['nix','develop','--command',str(exe),'frames',str(p/name),str(frames),scenario,'0xC0126',size],env)
(p/'inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('PHONE DRAFT READABILITY CHAIN PASS',flush=True)
