import hashlib,json,os,pathlib,shutil,subprocess,time
root=pathlib.Path.cwd();out=root/'target/verification/cave-adoption-r6-native-2026-11-06';build=root/'target/verification/cave-adoption-r6-2026-11-06'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def manifest(folder):return {str(p.relative_to(folder)):sha(p)for p in sorted(folder.rglob('*'))if p.is_file()}
source=json.loads((build/'inputs-after.json').read_text());assert source==json.loads((build/'inputs-before.json').read_text())
for name,value in source.items():assert sha(root/name)==value,name
assert sha(build/'capture')==json.loads((build/'binary.json').read_text())['sha256']
shutil.copy2(build/'capture',out/'capture')
assets=manifest(root/'assets');assert assets['models/environment_demon_cavern.glb']=='7cc6fe0dfbda094226f5f051492f8aee8c4a449fc90844c574cedf36995b7176'
clone=out/'isolated-assets';assert not clone.exists();shutil.copytree(root/'assets',clone);shutil.copy2(root/'assets/models/environment_demon_cavern.glb',clone/'models/environment_demon_cavern_preview.glb');expected=dict(assets);expected['models/environment_demon_cavern_preview.glb']=assets['models/environment_demon_cavern.glb'];assert manifest(clone)==expected
(out/'asset-manifest.json').write_text(json.dumps(expected,indent=2)+'\n');(out/'source-inputs.json').write_text(json.dumps(source,indent=2)+'\n')
results=[]
cases=[]
for locale,size,language in [('en-desktop','1280x720','en'),('zh-compact','667x375','zh-CN'),('en-portrait','390x844','en'),('zh-phone','375x667','zh-CN')]:
 for scenario,frames,saved in [('demon-cave',320,'0,29,60,90,180,295,319'),('den-preview',400,'0,40,90,100,130,180,240,350,399'),('camera-framing',260,'0,29,58,89,118,149,178,209,238,259'),('camera-framing-touch',260,'0,29,58,89,118,149,178,209,238,259')]:
  cases.append((locale+'-'+scenario,size,language,scenario,frames,saved))
for scenario in ('hit-feedback','hit-feedback-8x'):
 cases.append(('en-desktop-'+scenario,'1280x720','en',scenario,180,'0,8,9,10,11,12,14,18,60,89,90,119,120,121,122,149,150,152,165,179'))
for name,size,language,scenario,frames,saved in cases:
 case=out/name;case.mkdir();state=case/'state';state.mkdir()
 env={key:value for key,value in os.environ.items()if not key.startswith('ROGUE_')}
 env.update(ROGUE_CAPTURE_ASSET_ROOT=str(clone),ROGUE_CAPTURE_SAVE_FRAMES=saved,ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_COMMANDER_LANG=language,XDG_CONFIG_HOME=str(state/'config'),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
 if scenario.startswith('hit-feedback'):env['ROGUE_CAPTURE_SHAKE_VOLLEY']='1'
 args=['nix','develop','--command',str(out/'capture'),'frames',str(case/'frames'),str(frames),scenario,'0xA11CE',size]
 print('STAGE',name,'begin',flush=True);start=time.monotonic()
 with (case/'game.log').open('w')as log:result=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
 results.append({'case':name,'size':size,'language':language,'scenario':scenario,'frames':frames,'command':args,'exit':result.returncode,'seconds':time.monotonic()-start});(out/'results.json').write_text(json.dumps(results,indent=2)+'\n')
 text=(case/'game.log').read_text();assert result.returncode==0,name
 if scenario=='demon-cave':
  assert '[capture] PASS demon cave quiet:'in text and '[capture] PASS demon-cave: authored production cavern'in text
 elif scenario=='den-preview':
  assert '[capture] PASS den-preview:'in text
 elif scenario.startswith('hit-feedback'):
  assert '[capture] PASS dense-hit impact peak and exact production recovery'in text
  assert '[capture] PASS ordinary contact does not inherit critical strength'in text
  assert '[capture] PASS heavy-kill sequence'in text
  assert text.count('[capture] PASS 100 critical contacts use one camera impulse')==2
  assert '[capture] synthetic-hit-time frame=10'in text and '[capture] synthetic-hit-time frame=165'in text
 else:
  assert '[capture] PASS camera-framing:'in text and text.count('[capture] CAMERA_CORNER ')==4
 if scenario=='camera-framing-touch':
  assert '[capture] CAMERA_INPUT_MODE actual=Touch'in text and text.count('[capture] CAMERA_TOUCH ')==2
 assert f'wrote {frames} simulation frames'in text and 'ERROR'not in text and 'panicked'not in text
 assert len(list((case/'frames').glob('*.png')))==len(saved.split(','))
 for path,value in source.items():assert sha(root/path)==value,path
 assert manifest(root/'assets')==assets and manifest(clone)==expected
 print('STAGE',name,'technical PASS; visual/input acceptance separate',flush=True)
print('NATIVE_TECHNICAL_PASS four sizes/four cave-camera scenarios plus two impact fixtures5320updates; inspect saved pixels and speed trace before acceptance',flush=True)
