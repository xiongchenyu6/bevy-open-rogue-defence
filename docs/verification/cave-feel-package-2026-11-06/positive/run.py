import pathlib,json,hashlib,subprocess,time
r=pathlib.Path.cwd();o=pathlib.Path(__file__).resolve().parent;p=r/'target/desktop-packages-cave-feel-r3-nov6/rogue-commander-linux'
sha=lambda x:hashlib.sha256(x.read_bytes()).hexdigest()
v=json.loads((p/'version.json').read_text());assert v['git_commit']=='932fba4d3f4a172e9c7421be1b191bb6b989127e'
for f in json.loads((p/'package-manifest.json').read_text())['files']:assert sha(p/f['file'])==f['sha256']
for n,h in json.loads((o/'harness-provenance.json').read_text())['files'].items():assert sha(o/'harness'/n)==h
out=o/'output';out.mkdir()
a=['docker','run','--rm','--name','rogue-cave-feel-smoke-r3-nov6','--network','none','--mount',f'type=bind,src={p},dst=/game,readonly','--mount',f'type=bind,src={out},dst=/verify','--mount',f'type=bind,src={o/"harness"},dst=/harness,readonly','--env','PACKAGE_COMMIT='+v['git_commit'],'--workdir','/tmp','sha256:0d4cccee982b31492e0f7b01a50afc1a0ec710d3166dab22e02322ce64832715','python3','/harness/smoke_runtime.py']
(o/'executed-command.json').write_text(json.dumps(a,indent=2)+'\n');start=time.monotonic()
try:
 with (o/'runtime.log').open('w')as log:q=subprocess.run(a,stdout=log,stderr=subprocess.STDOUT,timeout=600)
 (o/'process.json').write_text(json.dumps({'exit':q.returncode,'seconds':time.monotonic()-start},indent=2)+'\n');assert q.returncode==0
 assert json.loads((out/'result.json').read_text())['normal_quit_twice']is True
 print('STRICT_SDK_SMOKE_PASS',flush=True)
finally:subprocess.run(['docker','stop','-t','15','rogue-cave-feel-smoke-r3-nov6'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
