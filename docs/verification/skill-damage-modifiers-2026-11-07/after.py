import pathlib,subprocess,hashlib,json,os,shutil
r=pathlib.Path.cwd();p=r/'target/verification/skill-damage-modifiers-2026-11-07'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'after-inputs.json').write_text(json.dumps(a,indent=2))
for name in ['skill_details.rs','skill_details/tests.rs','progression_ui.rs','progression.rs','resources.rs','skills.rs']:
 shutil.copy2(r/'src/game'/name,p/('after-'+name.replace('/','-')))
def run(name,cmd,env=None):
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}));assert x.returncode==0,(name,x.returncode)
 assert inputs()==a
 print(name,'TERMINAL PASS',flush=True)
run('focused-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','skill_details','-j2','--','--nocapture'])
run('library-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2'])
run('build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'capture';shutil.copy2(r/'target/market-visual/debug/capture',exe);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for lang in ['en','zh']:
 name='after-'+lang;env=os.environ.copy();env.update(EXPECT_CROWN_DETAIL_FIXED='1',EXPECT_SKILL_DAMAGE_MODIFIERS='1',ROGUE_COMMANDER_LANG=lang,RUST_LOG='info,rogue_commander::projectile_hit=trace',ROGUE_CAPTURE_SAVE_FRAMES='100,180,225,330,450,479',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
 run(name,['nix','develop','--command',str(exe),'frames',str(p/name),'480','crown-damage-preview','0xC0126','1280x720'],env)
 run(name+'-verify',['python3','target/verification/skill-preview-modifiers-implementation-2026-11-06/producer-fixture/verify-producer.py',str(p/(name+'.log')),'--modifiers','1','--frames','480'])
for stage in ['before','after']:
 for lang in ['en','zh']:
  name=stage+'-portrait-'+lang;env=os.environ.copy();env.update(EXPECT_CROWN_DETAIL_FIXED='1',EXPECT_SKILL_DAMAGE_MODIFIERS=str(int(stage=='after')),ROGUE_COMMANDER_LANG=lang,RUST_LOG='info,rogue_commander::projectile_hit=trace',ROGUE_CAPTURE_SAVE_FRAMES='100,180,225,330,450,479',VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(name+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(name+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(name+'-audio.save')))
  binary=p/'before-capture' if stage=='before' else exe
  run(name,['nix','develop','--command',str(binary),'frames',str(p/name),'480','crown-damage-preview','0xC0126','720x1280'],env)
  run(name+'-verify',['python3','target/verification/skill-preview-modifiers-implementation-2026-11-06/producer-fixture/verify-producer.py',str(p/(name+'.log')),'--modifiers',str(int(stage=='after')),'--frames','480'])
run('clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
(p/'after-inputs-after.json').write_text(json.dumps(inputs(),indent=2))
print('AFTER MODIFIERS CHAIN TERMINAL PASS',flush=True)
