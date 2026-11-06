"""Verify archived build/smoke and still-retained immutable local artifact, no game run."""
from pathlib import Path
import hashlib,io,json,subprocess,sys
P=Path(__file__).resolve().parent;ROOT=P.parents[2];COMMIT='0c12230d3bf25c2b1a070c80950e5388a3b7b2ec'
load=lambda f:json.loads((P/f).read_text());sha=lambda b:hashlib.sha256(b).hexdigest()
assert load('build-process.json')['exit']==0 and load('smoke-process.json')['exit']==0
assert 'POLISH_NEXT_PACKAGE_SMOKE_PASS' in (P/'manager.log').read_text() and 'DESKTOP_RUNTIME_PASS' in (P/'smoke.log').read_text()
v=load('version.json');assert v['git_commit']==COMMIT and v['linux_linkage']['checked'] and v['linux_linkage']['nix_dependencies']==[]
r=load('smoke/result.json');assert all(r[k] is True for k in ('package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle'))
args=load('smoke-process.json')['args'];assert args[args.index('--network')+1]=='none';assert any('dst=/game,readonly' in a for a in args) and any('dst=/harness,readonly' in a for a in args)
a=load('input-audit-before.json');b=load('input-audit-after.json');assert a['source_commit']==b['source_commit']==COMMIT and a['files']==b['files'] and len(a['files'])==525
assert set(a['allowed_untracked_art_tools'])=={'tools/build_demon_cavern.py','tools/verify_demon_cavern.py'}
names=list(a['files']);data=subprocess.check_output(['git','cat-file','--batch'],input=''.join(COMMIT+':'+n+'\n'for n in names).encode(),cwd=ROOT);stream=io.BytesIO(data)
for n in names:
 h=stream.readline().split();assert h[1]==b'blob';body=stream.read(int(h[2]));assert stream.read(1)==b'\n';assert sha(body)==a['files'][n],n
for name,h in load('harness-provenance.json')['files'].items():
 body=(P/'smoke/harness-source'/name).read_bytes();assert sha(body)==h
 assert body==subprocess.check_output(['git','show',COMMIT+':tools/desktop/'+name],cwd=ROOT)
assert load('root-artifact-check.json')['all_file_bytes_and_tar_metadata_pass'] is True
assert load('root-artifact-check.json')['manifest_files']==889
# Retained root script independently rechecks actual binary, all890 file bytes and tar metadata.
subprocess.run([sys.executable,str(P/'root-verify-artifact.py')],cwd=ROOT,check=True)
actual=ROOT/load('artifact.json')['package_directory'];assert (actual/'package-manifest.json').read_bytes()==(P/'package-manifest.json').read_bytes()
assert (actual/'version.json').read_bytes()==(P/'version.json').read_bytes()
for f in ['smoke/battle.png','smoke/restart-battle-chinese.png']:assert (P/f).is_file()
if (P/'files.sha256.json').exists():
 for f,h in load('files.sha256.json').items():assert sha((P/f).read_bytes())==h,f
print('POLISH_NEXT_PACKAGE_ARCHIVE_VERIFIED source='+COMMIT+' build0 smoke0 files889 inputs525; no new-source chapter-clear claim')
