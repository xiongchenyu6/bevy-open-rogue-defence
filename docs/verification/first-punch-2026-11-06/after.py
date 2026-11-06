import pathlib,subprocess,json,hashlib,shutil,os
root=pathlib.Path.cwd();p=root/'target/verification/first-punch-r2-2026-11-06'
def inputs():
 return {str(f.relative_to(root)):hashlib.sha256(f.read_bytes()).hexdigest() for f in sorted([root/'Cargo.toml',root/'Cargo.lock',*root.glob('src/**/*.rs'),*root.glob('assets/**/*')]) if f.is_file()}
a=inputs();(p/'after-inputs.json').write_text(json.dumps(a,indent=2)+'\n')
with (p/'after-build.log').open('w') as log:r=subprocess.run(['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'],stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0 and inputs()==a
exe=p/'after-capture';shutil.copy2(root/'target/market-visual/debug/capture',exe);(p/'after-binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest()+'\n')
state=p/'state-after';state.mkdir(exist_ok=True)
env=os.environ.copy();env.update(ROGUE_CAPTURE_HIT_MATERIAL_TRACE='1',ROGUE_CAPTURE_HIT_PUNCH_LOW_FPS='1',EXPECT_HIT_FLASH='1',EXPECT_HIT_PUNCH_FIRST_FRAME='1',ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(0,40))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
with (p/'after-native.log').open('w') as log:r=subprocess.run(['nix','develop','--command',str(exe),'frames',str(p/'after'),'210','attack-sword','0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0 and inputs()==a
text=(p/'after-native.log').read_text();assert 'HIT_PUNCH_FIRST' in text and 'visible=true real_delta=0.16' in text and 'HIT_MATERIAL_PASS' in text and 'PASS: real basic attack damage' in text
print('AFTER native PASS: actual contact first squash visible',flush=True)
for name,cmd in [('after-tests',['cargo','test','--locked','--release','--lib','-j2','--','--nocapture']),('clippy',['cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])]:
 with (p/(name+'.log')).open('w') as log:r=subprocess.run(['nix','develop','--command',*cmd],stdout=log,stderr=subprocess.STDOUT)
 (p/(name+'-exit.txt')).write_text(str(r.returncode)+'\n');print(name,'terminal',r.returncode,flush=True)
 assert r.returncode==0 and inputs()==a
(p/'after-inputs-after.json').write_text(json.dumps(inputs(),indent=2)+'\n')
print('TERMINAL AFTER PASS',flush=True)
