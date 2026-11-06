from pathlib import Path
import os,json,subprocess,hashlib,shutil,time
root=Path.cwd();out=root/'target/verification/demon-den-front-camera-2026-11-06';assets=root/'target/verification/demon-den-native-preview-r31-2026-11-06/assets';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(args,name,env=None,timeout=900):
 start=time.monotonic()
 with (out/name).open('w')as log:r=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=timeout)
 (out/(name+'.process.json')).write_text(json.dumps({'exit':r.returncode,'elapsed':time.monotonic()-start,'args':args},indent=2)+'\n');assert r.returncode==0,name
run(['nix','develop','--command','rustfmt','--edition','2024','--config','skip_children=true','src/bin/capture/demon_cave.rs'],'format.log')
files=sorted([*root.joinpath('src').rglob('*.rs'),*(p for p in root.joinpath('assets').rglob('*')if p.is_file()),root/'Cargo.toml',root/'Cargo.lock']);before={str(p.relative_to(root)):sha(p)for p in files};(out/'inputs-before.json').write_text(json.dumps(before,indent=2)+'\n')
manifest=json.loads((assets.parent/'asset-manifest.json').read_text())
for row in manifest['files']:assert sha(assets/row['file'])==row['sha256']
(out/'asset-reference.json').write_text(json.dumps({'asset_root':str(assets),'manifest':manifest},indent=2)+'\n')
flags=['--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']
for task,extra in [('clippy',['--','-D','warnings']),('build',[])]:run(['nix','develop','--command','cargo',task,*flags,*extra],task+'.log');print('PASS',task,flush=True)
assert before=={str(p.relative_to(root)):sha(p)for p in files};(out/'inputs-after-build.json').write_text(json.dumps(before,indent=2)+'\n');shutil.copy2(root/'target/market-visual/debug/capture',out/'capture');(out/'binary.json').write_text(json.dumps({'sha256':sha(out/'capture')},indent=2)+'\n')
state=out/'state';state.mkdir(exist_ok=True);env={k:v for k,v in os.environ.items()if not k.startswith('ROGUE_')};env.update(VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAPTURE_ASSET_ROOT=str(assets),ROGUE_CAPTURE_SAVE_FRAMES='30,90,120,180,240,300,350,399',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
run(['nix','develop','--command',str(out/'capture'),'frames',str(out/'front'),'400','den-preview-front','0xA11CE','1280x720'],'front.log',env,240)
text=(out/'front.log').read_text();assert 'PASS den-preview:'in text and 'camera=front-side proposal'in text and 'panicked at'not in text and 'ERROR'not in text;assert len(list((out/'front').glob('frame*.png')))==8
assert before=={str(p.relative_to(root)):sha(p)for p in files}
for row in manifest['files']:assert sha(assets/row['file'])==row['sha256']
(out/'result.json').write_text(json.dumps({'execution_passed':True,'scope':'Capture-only camera proposal: eye(-6,24,21),target(2,0,0),height24. Frozen R31 candidate. Production camera/geometry unchanged; visual approval pending.'},indent=2)+'\n');print('FRONT_CAMERA_NATIVE_EXECUTION_PASS',flush=True)
