import pathlib,json,subprocess,hashlib,shutil,os
root=pathlib.Path.cwd();out=root/'target/verification/melee-trails-growth-2026-11-06'
def inputs():
 files=list((root/'src').rglob('*.rs'))+list((root/'assets').rglob('*'))+[root/'Cargo.toml',root/'Cargo.lock']
 return {str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(files) if p.is_file()}
a=inputs();(out/'inputs-before.json').write_text(json.dumps(a,indent=2)+'\n')
commands=[('capture-clippy.log',['nix','develop','--command','cargo','clippy','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0','--','-D','warnings']),('capture-build.log',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])]
for name,args in commands:
 with (out/name).open('w') as log:r=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT)
 assert r.returncode==0,name
 print('PASS',name,flush=True)
b=inputs();(out/'inputs-after.json').write_text(json.dumps(b,indent=2)+'\n');assert a==b
exe=out/'capture';shutil.copy2(root/'target/market-visual/debug/capture',exe)
(out/'binary.json').write_text(json.dumps(dict(binary=str(exe),sha256=hashlib.sha256(exe.read_bytes()).hexdigest(),scope='Capture-only additional mappings; production runtime unchanged from acceptedfadecandidate'),indent=2)+'\n')
for kind in ['void','daggers']:
 state=out/('state-'+kind);state.mkdir(exist_ok=True)
 env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,list(range(4,81,2))+[120,160,200,230,249])),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
 with (out/(kind+'.log')).open('w') as log:r=subprocess.run(['nix','develop','--command',str(exe),'frames',str(out/kind),'250','weapon-growth-'+kind+'-upgraded','0xA11CE','1280x720'],env=env,stdout=log,stderr=subprocess.STDOUT)
 text=(out/(kind+'.log')).read_text();assert r.returncode==0 and 'PASS weapon-growth-'+kind+'-upgraded' in text and 'panicked' not in text,kind
 print('PASS actual level40 growth',kind,flush=True)
print('TERMINAL GROWTH PASS',flush=True)
