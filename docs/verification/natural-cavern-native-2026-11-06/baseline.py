import hashlib,json,os,pathlib,subprocess,time
root=pathlib.Path.cwd();out=root/'target/verification/natural-cavern-native-2026-11-06';case=out/'original-den-baseline';case.mkdir();state=case/'state';state.mkdir();source=json.loads((out/'source-inputs.json').read_text())
for n,h in source.items():assert hashlib.sha256((root/n).read_bytes()).hexdigest()==h,n
clone=out/'isolated-assets';expected=json.loads((out/'asset-manifest.json').read_text())
for n,h in expected.items():assert hashlib.sha256((clone/n).read_bytes()).hexdigest()==h,n
env=os.environ.copy();env.update(ROGUE_CAPTURE_ASSET_ROOT=str(clone),ROGUE_CAPTURE_SAVE_FRAMES='0,29,59,90,180,295,319',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
cmd=['nix','develop','--command',str(out/'capture'),'frames',str(case/'frames'),'320','demon-cave','0xA11CE','1280x720'];start=time.monotonic()
with (case/'game.log').open('w') as log:r=subprocess.run(cmd,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
(case/'result.json').write_text(json.dumps(dict(command=cmd,exit=r.returncode,seconds=time.monotonic()-start,scope='Original den, production camera, controlled quiet then real SpawnDirector waves; not the candidate actor fixture'),indent=2)+'\n')
text=(case/'game.log').read_text();assert r.returncode==0 and '[capture] PASS demon cave quiet:'in text and '[capture] PASS demon-cave:'in text and 'ERROR'not in text and 'panicked'not in text
assert 'wrote 320 simulation frames'in text and len(list((case/'frames').glob('*.png')))==7
for n,h in source.items():assert hashlib.sha256((root/n).read_bytes()).hexdigest()==h,n
for n,h in expected.items():assert hashlib.sha256((clone/n).read_bytes()).hexdigest()==h,n
print('BASELINE_ORIGINAL_DEN_PASS full quiet and actual wave controls, no art acceptance claim',flush=True)
