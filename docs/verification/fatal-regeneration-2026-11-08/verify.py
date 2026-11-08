import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['before/result.json'])['returncode']==101 and b'0 passed; 1 failed' in raw['before/tests.log']
assert json.loads(raw['after/library-result.json'])['returncode']==101 and b'790 passed; 1 failed; 1 ignored' in raw['after/library.log']
assert json.loads(raw['after-r2/interruption.json'])['validation_complete'] is False
assert 'after-r2/result.json' not in raw
for n in ['result.json','library-result.json','clippy-result.json','sim-build-result.json','capture-build-result.json','native-result.json']:assert json.loads(raw['after-r3/'+n])['returncode']==0
assert b'2 passed; 0 failed' in raw['after-r3/tests.log'] and b'791 passed; 0 failed; 1 ignored' in raw['after-r3/library.log']
assert b'ordinary_healing_requires_a_surviving_health_pool ... ok' in raw['after-r3/library.log']
pat=r'FATAL_REGEN regeneration=(true|false) prior_deaths=(\d+)(?: revival_seed=(true|false))? health=([\d.]+) alive=(true|false) deaths=(\d+) finished=(true|false) victory=(true|false)'
def variants(stage):
 rows=re.findall(pat,raw[stage+'/tests.log'].decode());return {(reg,int(prior),seed or 'false'):(float(hp),alive,int(deaths),finish,victory) for reg,prior,seed,hp,alive,deaths,finish,victory in rows}
b=variants('before');a=variants('after-r3');assert len(b)==4 and len(a)==5
for prior in [0,2]:
 expected=(0.,'false',prior+1,'true' if prior==2 else 'false','false')
 assert b[('false',prior,'false')]==expected
 assert b[('true',prior,'false')]==(.16662598,'true',prior,'false','false')
 assert a[('false',prior,'false')]==a[('true',prior,'false')]==expected
assert a[('true',2,'true')]==(453.6,'true',2,'false','false')
mp=json.loads(raw['after-r3/inputs.json']);assert len(mp)==211
join=json.loads((D/'source-join.json').read_text());assert len(join['source_files'])==4
for n in join['source_files']:assert sha((D/'source'/n).read_bytes())==mp[n]
for n in ['src/game/bosses.rs','src/game/combat/decision_phase_tests.rs']:assert sha(raw['before/source/'+n])==json.loads(raw['before/inputs.json'])[n]
env=json.loads(raw['after-r3/isolated-environment.json']);assert set(env)=={'XDG_CONFIG_HOME','ROGUE_CAMPAIGN_SAVE','ROGUE_AUDIO_SAVE','ROGUE_TUTORIAL_SAVE'}
assert raw['after-r3/host-profile-before.sha256'].strip()==raw['after-r3/host-profile-after.sha256'].strip()==b'060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
assert not any('observed-default-campaign.save' in n for n in m)
native=raw['after-r3/native.log'];assert b'wrote 400 simulation frames' in native and b'third death settles defeat' in native
review=json.loads(raw['after-r3/root-reviewed.json']);assert len(review['frames'])==7
for n in review['frames']:assert (D/'frames/after-r3/native'/n).exists()
state=(D/'unit-state.txt').read_text();assert 'Result=success' in state and 'ExecMainStatus=0' in state
print('PASS exact archive/raw bytes,actual fatal-regeneration negative,final2/791+1/Clippy/native,isolation and death/revival controls.')
