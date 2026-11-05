from pathlib import Path
import hashlib,json,sys
p=Path(__file__).resolve().parent;read=lambda n:json.loads((p/n).read_text());sha=lambda f:hashlib.sha256(f.read_bytes()).hexdigest()
for n,h in read('evidence-hashes.json').items():assert sha(p/n)==h,n
expected='d4540c44df479041fe4ff4fdb818f8fb2f280f99';pins=read('final-pins.json');assert pins['source_commit']==expected and pins['files_verified']==886
assert read('build-process.json')['exit']==read('smoke-process.json')['exit']==0
assert 'WALLET_BLADES_PACKAGE_SMOKE_PASS' in (p/'manager.log').read_text()
assert 'DESKTOP_RUNTIME_PASS' in (p/'smoke.log').read_text()
assert all(v is True for v in read('smoke/result.json').values() if isinstance(v,bool))
a,b=read('input-audit-before.json'),read('input-audit-after.json');assert a['source_commit']==b['source_commit']==expected and a['files']==b['files'] and len(a['files'])==516
allowed={'tools/build_demon_cavern.py','tools/verify_demon_cavern.py'}
assert set(a['allowed_untracked_art_tools'])<=allowed and set(b['allowed_untracked_art_tools'])<=allowed
assert all(n.startswith('tools/') for n in a['ignored_only_python_cache']+b['ignored_only_python_cache'])
assert read('version.json')['linux_linkage']['nix_dependencies']==[]
delta=read('package-delta.json');assert delta['old_files']==delta['new_files']==886 and not delta['added'] and not delta['removed'] and set(delta['changed'])=={'rogue_commander','version.json'}
helpers=read('harness-provenance.json');assert len(helpers['files'])==6 and 'forge_refresh.py' in helpers['files']
for n,h in helpers['files'].items():assert sha(p/'smoke/harness-source'/n)==h
assert read('root-artifact-verification.json')['passed']
if len(sys.argv)>1:
 q=Path(sys.argv[1]);assert sha(q/'rogue_commander')==pins['binary_sha256'] and sha(q/'package-manifest.json')==pins['manifest_sha256']
 for row in read('package-manifest.json')['files']:assert sha(q/row['file'])==row['sha256'],row['file']
print('PASS: d454 SDK/886file package receipts and offline shortflow only; not naturalForge/fullC6')
