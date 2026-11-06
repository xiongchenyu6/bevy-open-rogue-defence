import hashlib,json,os,pathlib,shutil,subprocess,time
root=pathlib.Path.cwd();out=root/'target/verification/natural-cavern-native-2026-11-06';build=root/'target/verification/den-wide-camera-2026-11-06';assetdir=root/'target/verification/cavern-closed-backing-r3-2026-11-06'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def manifest(folder):return {str(p.relative_to(folder)):sha(p) for p in sorted(folder.rglob('*')) if p.is_file()}
meta=json.loads((assetdir/'FINAL-CANDIDATE.json').read_text());candidate=root/meta['asset']
assert meta['sha256']==sha(candidate)=='d058912a9f56a9db4dfaf86d8ce8c03d08de855ed73faa556058a2ed38c900cf'
for n,h in meta['files_sha256'].items():assert sha(assetdir/n)==h,n
assert meta['actual_export_sat_collisions']==meta['actual_geometry_sat_collisions']==0
assert meta['postclear_topology']['boundary_edges']==meta['postclear_topology']['nonmanifold_edges']==0
assert len(meta['postclear_topology']['component_vertices'])==1
with (out/'independent-candidate-check.log').open('w') as log:
 r=subprocess.run(['python3',str(assetdir/'verify_export.py'),'--asset',meta['asset'],'--path-envelope','target/verification/demon-cavern-production-path-2026-11-06/production-animated-envelope.json'],stdout=log,stderr=subprocess.STDOUT,timeout=300)
assert r.returncode==0
(out/'candidate.json').write_text(json.dumps(meta,indent=2)+'\n')
source=json.loads((build/'build-inputs-after.json').read_text());assert source==json.loads((build/'build-inputs-before.json').read_text())
for n,h in source.items():assert sha(root/n)==h,n
exe=build/'capture';assert sha(exe)==json.loads((build/'binary.json').read_text())['sha256']
shutil.copy2(exe,out/'capture');exe=out/'capture'
original=manifest(root/'assets');tracked=set(subprocess.check_output(['git','ls-files','assets'],text=True).splitlines());assert {'assets/'+n for n in original}==tracked
clone=out/'isolated-assets';shutil.copytree(root/'assets',clone);shutil.copy2(candidate,clone/'models/environment_demon_cavern_preview.glb')
expected=dict(original);expected['models/environment_demon_cavern_preview.glb']=meta['sha256'];assert manifest(clone)==expected
(out/'asset-manifest.json').write_text(json.dumps(expected,indent=2)+'\n');(out/'source-inputs.json').write_text(json.dumps(source,indent=2)+'\n')
results=[]
for name,scenario in [('production-camera','den-preview'),('wide-camera-proposal','den-preview-wide')]:
 case=out/name;case.mkdir();state=case/'state';state.mkdir()
 env=os.environ.copy();env.update(ROGUE_CAPTURE_ASSET_ROOT=str(clone),ROGUE_CAPTURE_SAVE_FRAMES='0,40,90,100,130,180,240,350,399',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 cmd=['nix','develop','--command',str(exe),'frames',str(case/'frames'),'400',scenario,'0xA11CE','1280x720']
 print('STAGE',name,'start',flush=True);start=time.monotonic()
 with (case/'game.log').open('w') as log:r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
 text=(case/'game.log').read_text();entry=dict(case=name,command=cmd,exit=r.returncode,seconds=time.monotonic()-start)
 results.append(entry);(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
 assert r.returncode==0 and '[capture] PASS den-preview:' in text and 'panicked' not in text and 'ERROR' not in text
 assert 'wrote 400 simulation frames' in text
 assert len(list((case/'frames').glob('*.png')))==9
 assert manifest(clone)==expected and manifest(root/'assets')==original
 for n,h in source.items():assert sha(root/n)==h,n
 print('STAGE',name,'TECHNICAL PASS; visual review pending',flush=True)
print('NATIVE_BOTH_CASES_TECHNICAL_PASS NOT FINAL_ART_ACCEPTANCE',flush=True)
