import pathlib,subprocess,hashlib,json,os,shutil
r=pathlib.Path.cwd();p=r/'target/verification/crown-detail-2026-11-06'
def inputs():return {str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
a=inputs();(p/'before-inputs.json').write_text(json.dumps(a,indent=2))
shutil.copy2(r/'src/game/skill_details.rs',p/'before-skill_details.rs')
def run(name,cmd,expected=0,env=None):
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}))
 assert x.returncode==expected,(name,x.returncode,expected)
 assert inputs()==a
 print(name,'TERMINAL',x.returncode,flush=True)
run('before-regression',['nix','develop','--command','cargo','test','--locked','--release','--lib','crown_rank_six_evolution_preview_never_offers_a_downgrade','-j2','--','--nocapture'],101)
assert 'test result: FAILED. 0 passed; 1 failed;' in (p/'before-regression.log').read_text()
run('before-base',['nix','develop','--command','cargo','test','--locked','--release','--lib','base_rank_preview_advances_only_until_mastery','-j2','--','--nocapture'])
assert sum(line.startswith('BASE_RANK_DETAIL ') for line in (p/'before-base.log').read_text().splitlines())==384
run('before-build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'before-capture';shutil.copy2(r/'target/market-visual/debug/capture',exe);(p/'before-binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for lang in ['en','zh']:
 stage='before-'+lang;env=os.environ.copy();env.update(ROGUE_COMMANDER_LANG=lang,EXPECT_CROWN_DETAIL_FIXED='0',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,[*range(90,106),*range(170,186)])),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(stage+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(stage+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(stage+'-audio.save')))
 run(stage+'-native',['nix','develop','--command',str(exe),'frames',str(p/stage),'240','crown-detail','0xA11CE','1280x900'],env=env)
print('BEFORE CHAIN TERMINAL PASS: actual regression negative and native baseline captured',flush=True)
