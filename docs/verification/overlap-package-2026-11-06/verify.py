from pathlib import Path
import hashlib,json
root=Path(__file__).resolve().parent
def read(name):return json.loads((root/name).read_text())
for name,digest in read('evidence-hashes.json').items():
    assert hashlib.sha256((root/name).read_bytes()).hexdigest()==digest,name
artifact=read('artifact.json');version=read('version.json');check=read('package-check.json')
expected='f5ef6ff8f37ecab467b5ba0baadb6decd8d49aaf'
assert artifact['source_commit']==version['git_commit']==check['source_commit']==expected
assert artifact['binary_sha256']==version['binary_sha256']==check['binary_sha256']
assert version['release_package'] is True and version['profile']=='release'
link=version['linux_linkage'];assert link['checked'] and not link['nix_dependencies'] and not link['search_paths']
assert link['interpreter']==['/lib64/ld-linux-x86-64.so.2']
manifest=read('package-manifest.json')['files'];assert len(manifest)==check['files_verified']==886
assert len({row['file'] for row in manifest})==886
rack=next(row for row in manifest if row['file']=='assets/ui/armory/empty-rack.png')
assert rack['sha256']==check['new_armory_sha256']
assert not read('third-party-index.json')['missing_license_files']
for stage in ('build','smoke'):assert read(stage+'-process.json')['exit']==0
assert 'OVERLAP_PACKAGE_SMOKE_PASS' in (root/'sequence.log').read_text()
assert 'DESKTOP_RUNTIME_PASS' in (root/'smoke.log').read_text()
result=read('smoke/result.json')
for key in ('package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle'):assert result[key] is True,key
assert result['working_directory']=='/tmp'
for name in ('game.log','restart.log'):assert 'panicked at' not in (root/'smoke'/name).read_text()
for name,digest in read('harness-provenance.json')['files'].items():
    assert hashlib.sha256((root/'smoke/harness-source'/name).read_bytes()).hexdigest()==digest
print('PASS archived f5ef6ff standalone package: 886 files, build and offline read-only short flow; not full campaign/Steam/hardware acceptance')
