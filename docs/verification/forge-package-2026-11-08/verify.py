"""Portable proof integrity and actual process/result checks."""
import gzip
import hashlib
import json
import re
import sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def load(name):return json.loads((D/name).read_text())
def sha(data):return hashlib.sha256(data).hexdigest()
files=load('SHA256.json')
assert set(files)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for name,digest in files.items():assert sha((D/name).read_bytes())==digest,name
for name,row in load('raw-log-manifest.json').items():
    data=gzip.decompress((D/name).read_bytes());assert len(data)==row['bytes'] and sha(data)==row['sha256']
before=load('raw/input-audit-before.json');after=load('raw/input-audit-after.json')
assert before['files']==after['files'] and before['source_commit']==after['source_commit']
commit=before['source_commit'];assert len(commit)==40
assert load('raw/artifact.json')['source_commit']==commit
for name in ('build','smoke'):assert load('raw/'+name+'-process.json')['exit']==0
assert load('raw/tar-check.json')['exact_bytes'] is True and load('raw/tar-check.json')['normalized_metadata'] is True
harness=load('raw/harness-provenance.json');assert harness['commit']==commit
for name,digest in harness['files'].items():assert sha((D/'raw/smoke/harness-source'/name).read_bytes())==digest
r=load('raw/smoke/result.json')
for key in ('package_hashes_verified','no_source','no_rust','no_nix','chapter_recruit_battle','fullscreen_restored','language_saved_and_reloaded','normal_quit_twice','real_guide_after_readiness','tutorial_dismissal_saved','restart_skips_automatic_guide_and_runs_battle','public_portrait_skill_draw','portrait_skill_click_resumed_battle'):assert r[key] is True,key
assert 1<=r['portrait_skill_hands']<=8
assert b'DESKTOP_RUNTIME_PASS' in gzip.decompress((D/'raw/smoke.log.gz').read_bytes())
assert load('root-viewed-images.json')
for name in load('root-viewed-images.json'):assert (D/'frames'/name).is_file()
print('PASS package proof integrity, frozen input consensus, actual smoke and phone choice results')

proofs=r['battle_clock_proofs']
observations=r['battle_clock_observations']
assert load('raw/smoke/battle-progress.json')=={'proofs':proofs,'observations':observations}
for name,pair in proofs.items():
    first,second=pair['first'],pair['second']
    assert first in observations[name] and second in observations[name]
    assert first['completed_monotonic']<second['observed_monotonic']
    assert first['clock_seconds']>second['clock_seconds']>0
    for row in (first,second):
        assert not row['blockers'] and len(row['clock_reads'])>=2
        reads=[]
        for scale,value in row['clock_reads']:
            assert re.fullmatch(r'[0-9]{1,2}:[0-5][0-9]',value)
            minutes,seconds=map(int,value.split(':'));reads.append(minutes*60+seconds)
        assert set(reads)=={row['clock_seconds']}
    previous=0
    for row in observations[name]:
        assert row['observed_monotonic']>=previous
        assert row['completed_monotonic']>=row['observed_monotonic']
        previous=row['completed_monotonic']
        if row['clock_seconds'] is None:assert row.get('clock_error') or row['blockers']
receipt=load('raw/root-artifact-check.json')
assert receipt['result']=='PASS' and receipt['source_commit']==commit
assert receipt['source_inputs']==len(before['files'])
assert receipt['binary_sha256']==load('raw/artifact.json')['binary_sha256']
assert receipt['tar_sha256']==load('raw/artifact.json')['tar_sha256']
assert receipt['three_clock_pairs']=={name:[pair['first']['clock_seconds'],pair['second']['clock_seconds']] for name,pair in proofs.items()}
state=(D/'raw/unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
print('PASS strict clock pairs, all retained observations and root artifact receipt')

# Independent public-input forge preview, separate from master smoke.
preview_files=list((D/'raw/preview').rglob('forge-preview-result.json'))
assert len(preview_files)==1
pv=json.loads(preview_files[0].read_text())
for key in ('public_input','missing_materials_blocked','launcher_not_overlapping','battle_resumed'):assert pv[key] is True,key
assert pv['resource_injection'] is False
assert [(x['width'],x['height']) for x in pv['observations']]==[(1280,720),(360,800),(852,393)]
for row in pv['observations']:assert row['rows']
provenance=load('raw/preview/harness-provenance.json')
assert provenance['commit']==commit
assert sha((D/'raw/preview/harness-source/forge_preview_runtime.py').read_bytes())==provenance['derived_sha256']
assert sha((D/'raw/preview/harness-source/smoke_runtime.py').read_bytes())==provenance['base_smoke_sha256']
assert load('raw/preview/process-result.json')['returncode']==0
state=(D/'raw/preview/unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
preview=load('raw/preview/result.json');assert preview['normal_quit_twice'] is True
pp=preview['battle_clock_proofs'];po=preview['battle_clock_observations']
assert len(proofs)==3 and len(pp)==4
assert load('raw/preview/battle-progress.json')=={'proofs':pp,'observations':po}
for name,pair in pp.items():
    first,second=pair['first'],pair['second']
    assert first in po[name] and second in po[name]
    assert first['completed_monotonic']<second['observed_monotonic']
    assert first['clock_seconds']>second['clock_seconds']>0
    for row in (first,second):
        assert not row['blockers'] and len(row['clock_reads'])>=2
        reads=[]
        for scale,value in row['clock_reads']:
            assert re.fullmatch(r'[0-9]{1,2}:[0-5][0-9]',value)
            minutes,seconds=map(int,value.split(':'));reads.append(minutes*60+seconds)
        assert set(reads)=={row['clock_seconds']}
    previous=0
    for row in po[name]:
        assert row['observed_monotonic']>=previous
        assert row['completed_monotonic']>=row['observed_monotonic']
        previous=row['completed_monotonic']
        if row['clock_seconds'] is None:assert row.get('clock_error') or row['blockers']
assert (D/'frames/preview/forge-missing-blocked.png').read_bytes()==(D/'frames/preview/forge-preview-landscape.png').read_bytes()
check=load('raw/preview/root-preview-check.json');assert check['result']=='PASS' and check['source_commit']==commit
pair=pp['after-forge-preview'];assert check['after_forge_clock_pair']==[pair['first']['clock_seconds'],pair['second']['clock_seconds']]
print('PASS public forge preview, blocked byte equality, derived harness and all seven strict countdown pairs')
