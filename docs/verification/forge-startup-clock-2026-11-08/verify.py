import gzip,hashlib,json,re,sys
from pathlib import Path
D=Path(sys.argv[1]).resolve()
def sha(b):return hashlib.sha256(b).hexdigest()
m=json.loads((D/'SHA256.json').read_text());assert set(m)=={str(f.relative_to(D)) for f in D.rglob('*') if f.is_file() and f.name!='SHA256.json'}
for n,h in m.items():assert sha((D/n).read_bytes())==h,n
raw={}
for n,x in json.loads((D/'raw-manifest.json').read_text()).items():
 b=gzip.decompress((D/n).read_bytes());assert sha(b)==x['sha256'] and len(b)==x['bytes'];raw[n.removeprefix('raw/').removesuffix('.gz')]=b
assert json.loads(raw['r2/tests-result.json'])['returncode']==0
assert re.search(rb'Ran 115 tests',raw['r2/tests.log']) and re.search(rb'\nOK\s*$',raw['r2/tests.log'])
assert json.loads(raw['initial-test-failure/tests-result.json'])['returncode']==1
assert b'Ran 63 tests' in raw['initial-test-failure/tests.log'] and b'failures=1' in raw['initial-test-failure/tests.log']
assert json.loads(raw['r1-history/forge-process.json'])['returncode']==1
prov=json.loads((D/'provenance.json').read_text());assert len(prov['sources'])==3
for n,h in prov['sources'].items():assert sha((D/'source'/n).read_bytes())==h
assert sha((D/'failure-running.png').read_bytes())==prov['fixture']['sha256'] and prov['fixture']['expected_seconds']==1418
assert (D/'source/tools/desktop/test_forge_runtime.py').read_bytes()==(D/'initial-test-failure/source/tools/desktop/test_forge_runtime.py').read_bytes()
assert prov['new_fixture_test_unchanged_through_roi_fix'] is True
assert gzip.decompress((D/'source.patch.gz').read_bytes()).startswith(b'diff --git ')
print('PASS exact archive/raw bytes, natural attempt interruption,63-test reader failure,115-test success and3 source snapshots.')
