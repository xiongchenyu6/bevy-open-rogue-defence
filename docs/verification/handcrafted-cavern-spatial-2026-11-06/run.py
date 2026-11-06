import hashlib,json,os,pathlib,shutil,subprocess,time
root=pathlib.Path.cwd();out=root/'target/verification/handcrafted-cavern-native-2026-11-06';build=root/'target/verification/den-front-board-camera-2026-11-06';author=root/'target/verification/handcrafted-cave-prototype-flare-2026-11-06'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def manifest(folder):return {str(p.relative_to(folder)):sha(p)for p in sorted(folder.rglob('*'))if p.is_file()}
meta=json.loads((author/'FINAL-PROTOTYPE.json').read_text());asset=root/meta['asset']
assert sha(asset)==meta['sha256']=='d6fa53f5d700e5591e31dbe97225aa76475ada9838a13a6bdbf7020d8d3cd6ae'
assert meta['actual_path_audit']['collisions']==[] and meta['actual_path_audit']['height_bands']==96
cmd=['python3',str(author/'verify_prototype.py'),'--blockout','--asset',meta['asset'],'--path-envelope','target/verification/demon-cavern-production-path-2026-11-06/production-animated-envelope.json']
print('STAGE independent exported path check',flush=True)
with (out/'independent-export-check.log').open('w')as log:r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,timeout=300)
assert r.returncode==0
actual=json.loads((author/'strong-SAT-diagnostic.json').read_text());assert actual['asset_sha256']==meta['sha256'] and actual['path_audit']['collisions']==[] and actual['path_audit']['height_bands']==96
(out/'candidate.json').write_text(json.dumps(meta,indent=2)+'\n');(out/'independent-path.json').write_text(json.dumps(actual,indent=2)+'\n')
source=json.loads((build/'build-inputs-after.json').read_text());assert source==json.loads((build/'build-inputs-before.json').read_text())
for n,h in source.items():assert sha(root/n)==h,n
assert sha(build/'capture')==json.loads((build/'binary.json').read_text())['sha256'];shutil.copy2(build/'capture',out/'capture')
original=manifest(root/'assets');assert {'assets/'+n for n in original}==set(subprocess.check_output(['git','ls-files','assets'],text=True).splitlines())
clone=out/'isolated-assets';shutil.copytree(root/'assets',clone);shutil.copy2(asset,clone/'models/environment_demon_cavern_preview.glb');expected=dict(original);expected['models/environment_demon_cavern_preview.glb']=meta['sha256'];assert manifest(clone)==expected
(out/'asset-manifest.json').write_text(json.dumps(expected,indent=2)+'\n');(out/'source-inputs.json').write_text(json.dumps(source,indent=2)+'\n');results=[]
for name,scenario in [('production-camera','den-preview'),('front-board-proposal','den-preview-front-board')]:
 case=out/name;case.mkdir();state=case/'state';state.mkdir();env=os.environ.copy();env.update(ROGUE_CAPTURE_ASSET_ROOT=str(clone),ROGUE_CAPTURE_SAVE_FRAMES='0,40,90,100,130,180,240,350,399',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 cmd=['nix','develop','--command',str(out/'capture'),'frames',str(case/'frames'),'400',scenario,'0xA11CE','1280x720'];print('STAGE',name,'begin',flush=True);start=time.monotonic()
 with (case/'game.log').open('w')as log:r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
 results.append(dict(case=name,command=cmd,exit=r.returncode,seconds=time.monotonic()-start));(out/'results.json').write_text(json.dumps(results,indent=2)+'\n');text=(case/'game.log').read_text()
 assert r.returncode==0 and '[capture] PASS den-preview:'in text and 'wrote 400 simulation frames'in text and 'panicked'not in text and 'ERROR'not in text
 assert len(list((case/'frames').glob('*.png')))==9
 for n,h in source.items():assert sha(root/n)==h,n
 assert manifest(root/'assets')==original and manifest(clone)==expected
 print('STAGE',name,'technical PASS; visual/spatial review pending',flush=True)
print('BOTH_NATIVE_CASES_TECHNICAL_PASS grey prototype only, no final art/camera acceptance',flush=True)
