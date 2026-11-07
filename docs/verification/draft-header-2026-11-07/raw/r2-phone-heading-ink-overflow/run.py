import pathlib,subprocess,hashlib,json,os,shutil
r=pathlib.Path.cwd();p=r/'target/verification/draft-header-2026-11-07'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'inputs.json').write_text(json.dumps(a,indent=2))
for f in ['src/game/progression_ui.rs','src/game/progression_ui/layout.rs','src/bin/capture.rs','src/bin/capture/crown_detail.rs','src/bin/capture/draft_layout.rs']:
 q=p/'source'/f;q.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(r/f,q)
def run(name,cmd,env=None):
 with(p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}));assert x.returncode==0,(name,x.returncode)
 assert inputs()==a
 print(name,'TERMINAL PASS',flush=True)
run('library-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2'])
run('build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'capture';shutil.copy2(r/'target/market-visual/debug/capture',exe);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
old=r/'target/verification/skill-damage-modifiers-2026-11-07/capture';oldhash=hashlib.sha256(old.read_bytes()).hexdigest();(p/'before-binary.sha256').write_text(oldhash)
for stage,binary in [('after',exe)]:
 for size in ['1280x720','720x1280','844x390','360x800']:
  for lang in ['en','zh']:
   name=stage+'-'+size+'-'+lang;env=os.environ.copy();env.update(EXPECT_CROWN_DETAIL_FIXED='1',ROGUE_COMMANDER_LANG=lang,ROGUE_CAPTURE_SAVE_FRAMES='100,180,239',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
   if stage=='after':env['ROGUE_CAPTURE_DRAFT_LAYOUT']='1'
   run(name,['nix','develop','--command',str(binary),'frames',str(p/name),'240','crown-detail','0xC0126',size],env)
   text=(p/(name+'.log')).read_text();assert 'wrote 240 simulation frames' in text and 'PASS crown-detail:' in text
   if stage=='after':assert text.count('PASS draft-layout frame=')==2
run('clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
(p/'inputs-after.json').write_text(json.dumps(inputs(),indent=2));print('DRAFT HEADER CHAIN TERMINAL PASS',flush=True)
