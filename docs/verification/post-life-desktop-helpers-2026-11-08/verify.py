import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
host='060649ac32b34ce2cda231a9cf1ebcdfd8e8c12f0b0b3f94e527714f6993d911'
for i in range(1,9):
 label='r'+str(i);process=json.loads(raw[label+'/process.json']);prov=json.loads(raw[label+'/provenance.json']);assert process['returncode']==1
 assert process['host_profile_unchanged'] is True and process['harness_unchanged'] is True
 assert prov['host_profile_before']==process['host_profile_after']==host
 for n,h in prov['harness_files'].items():assert sha(raw[label+'/chapter6/harness-source/'+n])==h
assert b'FAILED (failures=13)' in raw['r6/helper-tests.log'] and b'Ran 67 tests' in raw['r6/helper-tests.log']
assert b'Ran 67 tests' in raw['r6/helper-tests-corrected.log'] and re.search(rb'\nOK\s*$',raw['r6/helper-tests-corrected.log'])
assert b'Ran 68 tests' in raw['r8/helper-tests-corrected.log'] and re.search(rb'\nOK\s*$',raw['r8/helper-tests-corrected.log'])
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==4
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
images=json.loads((D/'selected-images.json').read_text());assert len(images)==8
for n,x in images.items():assert sha((D/n).read_bytes())==x['sha256']
equipment=(D/'source/tools/desktop/campaign_equipment_runtime.py').read_text();clock=equipment.split('def battle_clock_reads(',1)[1].split('\ndef ',1)[0]
assert "'-threshold', '65%'" in clock
assert equipment.count("'-threshold', '65%'")==1
assert 'market_header_text(image, env)' in equipment
assert gzip.decompress((D/'source.patch.gz').read_bytes()).startswith(b'diff --git ')
assert not any('r9/' in n or n.endswith('.save') for n in m)
print('PASS exact archive/raw bytes,eight failed helper routes,13-failure/67/68-test history,source snapshots and unchanged host hashes.')
