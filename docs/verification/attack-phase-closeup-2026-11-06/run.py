import pathlib,subprocess,hashlib,json,os,shutil
r=pathlib.Path.cwd();p=r/'target/verification/attack-phase-closeup-2026-11-06'
a={str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()};(p/'inputs.json').write_text(json.dumps(a,indent=2))
def run(name,cmd,env=None):
 with (p/(name+'.log')).open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 (p/(name+'-result.json')).write_text(json.dumps({'command':cmd,'returncode':x.returncode}));assert x.returncode==0
 assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in a.items())
 print(name,'TERMINAL PASS',flush=True)
run('build',['nix','develop','--command','cargo','build','--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0'])
exe=p/'capture';shutil.copy2(r/'target/market-visual/debug/capture',exe);(p/'binary.sha256').write_text(hashlib.sha256(exe.read_bytes()).hexdigest())
for kind in ['ember','storm','sword']:
 env=os.environ.copy();env.update(ROGUE_CAPTURE_ATTACK_TRACE='1',ROGUE_CAPTURE_RELEASE_TRACE='1',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,range(0,61))),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAMPAIGN_SAVE=str(p/(kind+'-campaign.save')),ROGUE_TUTORIAL_SAVE=str(p/(kind+'-tutorial.save')),ROGUE_AUDIO_SAVE=str(p/(kind+'-audio.save')))
 run(kind,['nix','develop','--command',str(exe),'frames',str(p/kind),'210','attack-'+kind,'0xA11CE','1280x720'],env)
 text=(p/(kind+'.log')).read_text();assert 'wrote 210 simulation frames' in text and 'RELEASE_PHASE frame=210' in text and 'PASS: real basic attack damage' in text
run('clippy',['nix','develop','--command','cargo','clippy','--locked','--release','--all-targets','-j2','--','-D','warnings'])
(p/'inputs-after.json').write_text(json.dumps(a,indent=2))
print('CLOSEUP CHAIN TERMINAL PASS',flush=True)
