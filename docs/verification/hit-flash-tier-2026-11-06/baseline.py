import pathlib, subprocess, os, json, hashlib, shutil
r=pathlib.Path.cwd(); p=r/'target/verification/hit-flash-tier-2026-11-06'
inputs={str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()}
(p/'baseline-inputs.json').write_text(json.dumps(inputs,indent=2))
cmd=['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']
with (p/'baseline-build.log').open('w') as log: result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
assert result.returncode==0
assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in inputs.items())
exe=p/'baseline-capture'; shutil.copy2(r/'target/market-visual/debug/capture',exe)
(p/'baseline-binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
env=os.environ.copy(); env.update(ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(0,155))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/'baseline-campaign.save'),ROGUE_AUDIO_SAVE=str(p/'baseline-audio.save'),ROGUE_TUTORIAL_SAVE=str(p/'baseline-tutorial.save'))
with (p/'baseline-native.log').open('w') as log: result=subprocess.run(['nix','develop','--command',str(exe),'frames',str(p/'baseline'),'180','hit-feedback','0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
(p/'baseline-result.json').write_text(json.dumps({'returncode':result.returncode,'scope':'Controlled presentation input fixture, not real damage resolver or public gameplay acceptance'}))
assert result.returncode==0
assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in inputs.items())
print('BASELINE TERMINAL PASS',flush=True)
