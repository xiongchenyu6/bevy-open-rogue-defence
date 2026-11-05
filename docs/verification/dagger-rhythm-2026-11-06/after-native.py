import pathlib,subprocess,json,hashlib,shutil,os,time
root=pathlib.Path.cwd();out=root/'target/verification/dagger-rhythm-2026-11-06'
def receipt():
 files=[root/'Cargo.toml',root/'Cargo.lock']+list((root/'src').rglob('*.rs'))+list((root/'assets').rglob('*'))
 return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files) if p.is_file()}
a=receipt();(out/'after-inputs.json').write_text(json.dumps(a,indent=2)+'\n')
args=['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']
with (out/'after-build.log').open('w') as log:r=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
assert r.returncode==0
b=receipt();(out/'after-inputs-after.json').write_text(json.dumps(b,indent=2)+'\n');assert a==b,'Inputs changed during build'
exe=out/'after-capture';shutil.copy2(root/'target/market-visual/debug/capture',exe)
(out/'after-binary.json').write_text(json.dumps(dict(binary=str(exe),sha256=hashlib.sha256(exe.read_bytes()).hexdigest(),scope='Current source capture explicit cached build; pre/post inputs equal'),indent=2)+'\n')
print('AFTER build gate PASS',flush=True)
for kind in ['rift','tide','ember']:
 state=out/('state-after-'+kind);state.mkdir(exist_ok=True)
 env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,list(range(8,27))+[30,40,50,60,70])),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
 folder=out/('after-'+kind)
 with (out/('after-'+kind+'.log')).open('w') as log:r=subprocess.run(['nix','develop','--command',str(exe),'frames',str(folder),'210','attack-'+kind,'0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
 text=(out/('after-'+kind+'.log')).read_text();assert r.returncode==0 and 'PASS: real basic attack damage' in text and 'panicked' not in text,kind
 print('AFTER native PASS',kind,flush=True)
print('TERMINAL AFTER PASS',flush=True)
