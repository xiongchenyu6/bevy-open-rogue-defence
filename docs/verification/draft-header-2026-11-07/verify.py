"""Portable archive verification, independent geometry recomputation."""
import gzip
import hashlib
import importlib.util
import json
import re
import tempfile
from pathlib import Path
import sys
sys.dont_write_bytecode = True
D=Path(sys.argv[1]).resolve()
def sha(data):return hashlib.sha256(data).hexdigest()
def load(name):return json.loads((D/name).read_text())
def log(name):return gzip.decompress((D/'raw'/(name+'.log.gz')).read_bytes()).decode()
files=load('SHA256.json')
assert set(files)=={str(p.relative_to(D)) for p in D.rglob('*') if p.is_file() and p.name!='SHA256.json'}
for name,digest in files.items():assert sha((D/name).read_bytes())==digest,name
for name,record in load('raw-log-manifest.json').items():
    data=gzip.decompress((D/name).read_bytes());assert len(data)==record['bytes'] and sha(data)==record['sha256'],name
spec=importlib.util.spec_from_file_location('geometry',D/'verify_geometry.py');geometry=importlib.util.module_from_spec(spec);spec.loader.exec_module(geometry)
for stage in ('before','after'):
    for size in ('1280x720','720x1280','844x390','360x800'):
        for lang in ('en','zh'):
            name=f'{stage}-{size}-{lang}'
            assert load('raw/'+name+'-result.json')['returncode']==0
            text=log(name);assert 'wrote 240 simulation frames' in text and 'PASS crown-detail:' in text
            if stage=='after':
                with tempfile.TemporaryDirectory() as tmp:
                    path=Path(tmp)/'native.log';path.write_text(text);geometry.inspect(path,*map(int,size.split('x')))
for size in ('1280x720','360x800'):
    for scenario in ('immediate-skill','immediate-bond','immediate-skill-paid'):
        name=f'pointer-{size}-{scenario}'
        assert load('raw/'+name+'-result.json')['returncode']==0
        text=log(name);assert 'wrote 320 simulation frames' in text and 'PASS '+scenario+':' in text
        if scenario.endswith('-paid'):assert 'exact285→0' in text and 'next145 unaffordable' in text and 'heldclick no duplicate' in text
        else:assert 'three real mouse choices2/1/0' in text and 'held mouse no duplicate' in text
for name in ('library-tests','build','clippy'):assert load('raw/'+name+'-result.json')['returncode']==0
assert re.search(r'test result: ok\. 764 passed; 0 failed; 1 ignored;',log('library-tests'))
a=load('raw/inputs.json');b=load('raw/inputs-after.json');assert a==b and len(a)==394
for path in (D/'raw/source').rglob('*.rs'):
    name=str(path.relative_to(D/'raw/source'));assert sha(path.read_bytes())==a[name],name
assert len(load('raw/before-inputs.json'))==392
assert len(load('root-viewed-images.json'))==25
pointer=load('raw/pointer-inputs.json')
assert pointer==load('raw/pointer-inputs-after.json') and len(pointer)==394
assert {name for name in a if a[name]!=pointer[name]}=={'src/bin/capture/immediate_choice.rs'}
assert sha((D/'raw/pointer-immediate_choice.rs').read_bytes())==pointer['src/bin/capture/immediate_choice.rs']
for name in ('pointer-library-tests','pointer-build','pointer-clippy'):
    assert load('raw/'+name+'-result.json')['returncode']==0
assert re.search(r'test result: ok\. 764 passed; 0 failed; 1 ignored;',log('pointer-library-tests'))
for name in ('warm-360x800-en','warm-main-360x800-en'):
    assert load('raw/'+name+'-result.json')['returncode']==0
    text=log(name);assert 'wrote 240 simulation frames' in text and 'PASS crown-detail:' in text
    with tempfile.TemporaryDirectory() as tmp:
        path=Path(tmp)/'warm.log';path.write_text(text);geometry.inspect(path,360,800)
for name in ('unit-state.txt','pointer-unit-state.txt'):
    state=(D/'raw'/name).read_text()
    assert 'Result=success' in state and 'ExecMainStatus=0' in state
before=load('raw/before-provenance.json')
assert before['binary_sha256']==(D/'raw/before-binary.sha256').read_text().strip()
for directory in ('r1-compact-header-overflow','r2-phone-heading-ink-overflow','r3-cjk-heading-measure','pointer-r1-stale-next-price'):
    assert (D/'raw'/directory).is_dir() and any((D/'raw'/directory).rglob('*.log.gz'))

print('PASS draft header archive: 16 native cases, 6 pointer cases, independent geometry, exact fileset')
