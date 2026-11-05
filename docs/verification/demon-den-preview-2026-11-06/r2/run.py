from pathlib import Path
import os,sys,json,hashlib,subprocess,shutil,time
root=Path.cwd();out=root/'target/verification/demon-den-native-preview-r2-2026-11-06'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def write(name,data):(out/name).write_text(json.dumps(data,indent=2)+'\n')
def command(args,name,env=None,timeout=900):
    started=time.monotonic()
    try:
        with (out/name).open('w') as log:r=subprocess.run(args,stdout=log,stderr=subprocess.STDOUT,env=env,timeout=timeout)
        status={'exit':r.returncode,'elapsed':time.monotonic()-started,'args':args}
    except subprocess.TimeoutExpired:
        status={'timeout':True,'elapsed':time.monotonic()-started,'args':args}
    write(name+'.process.json',status)
    return status
args=['nix','develop','--command','rustfmt','--edition','2024','--config','skip_children=true','src/bin/capture.rs','src/bin/capture/demon_cave.rs']
assert command(args,'format.log')['exit']==0
files=sorted([*root.joinpath('src').rglob('*.rs'),*(p for p in root.joinpath('assets').rglob('*')if p.is_file()),root/'Cargo.toml',root/'Cargo.lock'])
inputs={str(p.relative_to(root)):sha(p)for p in files};write('inputs-before.json',inputs)
assets=out/'assets';manifest=json.loads((out/'asset-manifest.json').read_text())['files']
def check_assets():
    for row in manifest:assert sha(assets/row['file'])==row['sha256'],row['file']
check_assets()
base=['--locked','--bin','capture','-j1','--target-dir','target/market-visual','--config','profile.dev.opt-level=0','--config','profile.dev.package."*".opt-level=0','--config','profile.dev.debug=0']
for task,extra in [('clippy',['--','-D','warnings']),('build',[])]:
    status=command(['nix','develop','--command','cargo',task,*base,*extra],task+'.log')
    assert status.get('exit')==0,task
    print('PASS',task,flush=True)
assert inputs=={str(p.relative_to(root)):sha(p)for p in files}
write('inputs-after-build.json',inputs)
exe=out/'capture';shutil.copy2(root/'target/market-visual/debug/capture',exe)
write('binary.json',{'sha256':sha(exe),'scope':'Capture tool only; candidate assets cloned, shipped game and assets unchanged'})
for scenario,frames in [('den-preview',400),('den-preview-reframed',400),('demon-cave',320)]:
    env={k:v for k,v in os.environ.items()if not k.startswith('ROGUE_')};state=out/('state-'+scenario);state.mkdir(exist_ok=True)
    env.update(VK_ICD_FILENAMES='/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json',ROGUE_CAPTURE_SAVE_FRAMES=','.join(map(str,[30,90,120,180,240,300,350,399] if frames==400 else [30,90,120,180,240,300,319])),ROGUE_CAMPAIGN_SAVE=str(state/'campaign.save'),ROGUE_TUTORIAL_SAVE=str(state/'tutorial.save'),ROGUE_AUDIO_SAVE=str(state/'audio.save'))
    if scenario.startswith('den-preview'):env['ROGUE_CAPTURE_ASSET_ROOT']=str(assets)
    args=['nix','develop','--command',str(exe),'frames',str(out/scenario),str(frames),scenario,'0xA11CE','1280x720']
    status=command(args,scenario+'.log',env,240);text=(out/(scenario+'.log')).read_text()
    marker='PASS den-preview:' if scenario.startswith('den-preview')else 'PASS demon-cave:'
    assert status.get('exit')==0 and marker in text and 'panicked at' not in text and 'ERROR' not in text,scenario
    assert len(list((out/scenario).glob('frame*.png')))==(8 if frames==400 else 7),scenario
    print('PASS actual native',scenario,flush=True)
check_assets();assert inputs=={str(p.relative_to(root)):sha(p)for p in files}
write('result.json',{'execution_passed':True,'scope':'Actual Bevy candidate preview with cloned assets; two cameras and original entrance control. Visual quality remains unapproved, no candidate integration or earned-gameplay/performance claim.','candidate_sha256':manifest[-1]['sha256'],'cases':['den-preview','den-preview-reframed','demon-cave']})
print('NATIVE_DEN_PREVIEW_EXECUTION_PASS',flush=True)
