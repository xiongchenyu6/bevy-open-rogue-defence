import pathlib, subprocess, os, json, hashlib, shutil
r=pathlib.Path.cwd();p=r/'target/verification/hit-flash-tier-2026-11-06'
inputs={str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
(p/'after-inputs.json').write_text(json.dumps(inputs,indent=2))
def run(name,cmd,env=None):
 with (p/(name+'.log')).open('w') as log: result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':result.returncode}))
 assert result.returncode==0,(name,result.returncode)
 assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in inputs.items())
 print(name,'TERMINAL PASS',flush=True)
run('focused-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','hit_flash','-j2','--','--nocapture'])
run('tier-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','simultaneous_ordinary_and_critical_contacts','-j2','--','--nocapture'])
run('after-build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'after-capture';shutil.copy2(r/'target/market-visual/debug/capture',exe)
(p/'after-binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for stage, scenario, count in [('after','hit-feedback',180),('baseline-real','weapon-growth-sword-starter',250),('after-real','weapon-growth-sword-starter',250)]:
 binary=p/'baseline-capture' if stage=='baseline-real' else exe
 env=os.environ.copy();env.update(ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(0,155 if scenario=='hit-feedback' else 250))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(stage+'-campaign.save')),ROGUE_AUDIO_SAVE=str(p/(stage+'-audio.save')),ROGUE_TUTORIAL_SAVE=str(p/(stage+'-tutorial.save')))
 if scenario!='hit-feedback':env.update(ROGUE_CAPTURE_HIT_MATERIAL_TRACE='1',EXPECT_HIT_FLASH='1',ROGUE_CAPTURE_ATTACK_TRACE='1',RUST_LOG='info,rogue_commander::projectile_hit=trace')
 run(stage+'-native',['nix','develop','--command',str(binary),'frames',str(p/stage),str(count),scenario,'0xA11CE','1280x720'],env)
run('library-tests',['nix','develop','--command','cargo','test','--locked','--release','--lib','-j2'])
run('clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
(p/'after-inputs-after.json').write_text(json.dumps(inputs,indent=2))
print('AFTER CHAIN TERMINAL PASS',flush=True)
