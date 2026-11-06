import pathlib,json,hashlib,shutil,os,subprocess,time
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent;b=r/'target/verification/gate-cutaway-r3-2026-11-06'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def manifest(p):return {str(x.relative_to(p)):sha(x)for x in sorted(p.rglob('*'))if x.is_file()}
source=json.loads((b/'inputs-after.json').read_text());assert source==json.loads((b/'inputs-before.json').read_text())
for n,h in source.items():assert sha(r/n)==h,n
assert sha(b/'capture')==json.loads((b/'binary.json').read_text())['sha256'];shutil.copy2(b/'capture',o/'capture')
assets=manifest(r/'assets');clone=o/'isolated-assets';shutil.copytree(r/'assets',clone);assert manifest(clone)==assets
(o/'source-inputs.json').write_text(json.dumps(source,indent=2)+'\n');(o/'asset-manifest.json').write_text(json.dumps(assets,indent=2)+'\n')
results=[]
for label,size,language in [('en-desktop','1280x720','en'),('zh-compact','667x375','zh-CN'),('en-phone','375x667','en')]:
 for mode in ('gate-cutaway-opaque','gate-cutaway'):
  name=label+'-'+mode;case=o/name;case.mkdir();state=case/'state';state.mkdir()
  env={k:v for k,v in os.environ.items()if not k.startswith('ROGUE_')}
  env.update(ROGUE_CAPTURE_ASSET_ROOT=str(clone),ROGUE_CAPTURE_SAVE_FRAMES='0,60,120,180,240,300,360,420,480,540,579',ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_COMMANDER_LANG=language,XDG_CONFIG_HOME=str(state/'config'),VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json')
  args=['nix','develop','--command',str(o/'capture'),'frames',str(case/'frames'),'580',mode,'0xA11CE',size]
  print('STAGE',name,'begin',flush=True);start=time.monotonic()
  with(case/'game.log').open('w')as log:q=subprocess.run(args,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=360)
  results.append({'case':name,'size':size,'language':language,'mode':mode,'frames':580,'command':args,'exit':q.returncode,'seconds':time.monotonic()-start});(o/'results.json').write_text(json.dumps(results,indent=2)+'\n')
  log=(case/'game.log').read_text();assert q.returncode==0,name
  assert '[capture] PASS gate-cutaway:'in log and log.count('[capture] GATE_CUTAWAY ')==9
  assert 'wrote 580 simulation frames'in log and 'ERROR'not in log and 'panicked'not in log
  assert len(list((case/'frames').glob('*.png')))==11
  for n,h in source.items():assert sha(r/n)==h,n
  assert manifest(clone)==assets and manifest(r/'assets')==assets
  print('STAGE',name,'technical PASS; actual pixels still require review',flush=True)
print('GATE_CUTAWAY_NATIVE_TECHNICAL_PASS six cases3480updates; opaque control is not an old-binary run',flush=True)
