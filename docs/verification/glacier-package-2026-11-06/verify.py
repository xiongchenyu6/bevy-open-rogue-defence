from pathlib import Path
import hashlib,json,re,sys
p=Path(__file__).resolve().parent
m=json.loads((p/'SHA256.json').read_text());assert set(m)=={str(q.relative_to(p))for q in p.rglob('*')if q.is_file()and q.name!='SHA256.json'}
for n,h in m.items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==h,n
a=json.loads((p/'package/artifact.json').read_text());commit=a['source_commit'];assert commit=='bdd6660f16514a1ba1bd506c3a58f42916183e10'
for n in ['build-process.json','smoke-process.json']:assert json.loads((p/'package'/n).read_text())['exit']==0
before=json.loads((p/'package/input-audit-before.json').read_text());after=json.loads((p/'package/input-audit-after.json').read_text());assert before['files']==after['files']and len(before['files'])==537
r=json.loads((p/'smoke/result.json').read_text());assert all(v for v in r.values()if isinstance(v,bool))and r['normal_quit_twice']
for k in ['initial','restart']:
 d=r['battle_clock_proofs'][k];assert d['second']['clock_seconds']<d['first']['clock_seconds']
for n in ['game.log','restart.log']:assert not re.search(r'\bERROR\b',(p/'smoke'/n).read_text())
s=(p/'unit-status.txt').read_text();assert 'ActiveState=inactive'in s and 'Result=success'in s and 'ExecMainStatus=0'in s
h=json.loads((p/'package/harness-provenance.json').read_text());assert h['commit']==commit
for n,v in h['files'].items():assert hashlib.sha256((p/'harness'/n).read_bytes()).hexdigest()==v
rootcheck=json.loads((p/'package/root-artifact-check.json').read_text());assert rootcheck['game_source_matches_740_test_input_map']and rootcheck['all_tar_metadata_zero']
validated=p/'package/validated-test-inputs.json';assert hashlib.sha256(validated.read_bytes()).hexdigest()==rootcheck['test_input_manifest_sha256']
assert all(before['files'].get(n)==h for n,h in json.loads(validated.read_text()).items())
assert all(x['exit']==0 for x in json.loads((p/'package/source-test-processes.json').read_text()))
assert 'ok. 740 passed; 0 failed; 1 ignored'in(p/'package/source-tests.log').read_text()
print('Archived bdd candidate + offline public short flow PASS; broader acceptance remains open')
if '--artifacts'in sys.argv:
 import tarfile
 root=p.parents[2];d=root/a['package_directory'];t=root/a['tar'];sha=lambda q:hashlib.sha256(q.read_bytes()).hexdigest()
 assert sha(d/'rogue_commander')==a['binary_sha256']and sha(t)==a['tar_sha256']and t.stat().st_size==a['tar_bytes']
 rows=json.loads((p/'package/package-manifest.json').read_text())['files'];assert len(rows)==892
 for row in rows:assert sha(d/row['file'])==row['sha256'],row['file']
 with tarfile.open(t)as archive:
  files=[m for m in archive.getmembers()if m.isfile()];assert len(files)==893
  for m in archive.getmembers():assert m.uid==m.gid==m.mtime==0
  for m in files:
   n=Path(m.name).relative_to('rogue-commander-linux');assert archive.extractfile(m).read()==(d/n).read_bytes()
 print('Local artifacts PASS: 892 manifest files + 893 tar files')
