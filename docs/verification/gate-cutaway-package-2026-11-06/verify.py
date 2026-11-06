import pathlib,json,hashlib,re,sys
p=pathlib.Path(__file__).resolve().parent
for n,h in json.loads((p/'SHA256.json').read_text()).items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==h,n
meta=json.loads((p/'package/artifact.json').read_text());commit=meta['source_commit'];assert commit=='f205e965ac61bc7494457e5f3de0f56ce3cf164e'
for n in ('build-process.json','smoke-process.json'):assert json.loads((p/'package'/n).read_text())['exit']==0
before=json.loads((p/'package/input-audit-before.json').read_text());after=json.loads((p/'package/input-audit-after.json').read_text());assert before['files']==after['files'] and len(before['files'])==534
r=json.loads((p/'smoke/result.json').read_text());assert all(v for v in r.values()if isinstance(v,bool)) and r['normal_quit_twice'] is True
for key in ('initial','restart'):
 d=r['battle_clock_proofs'][key];assert d['second']['clock_seconds']<d['first']['clock_seconds']
for n in ('game.log','restart.log'):assert not re.search(r'\bERROR\b',(p/'smoke'/n).read_text())
u=(p/'unit-status.txt').read_text();assert 'Result=success'in u and 'ExecMainStatus=0'in u
h=json.loads((p/'package/harness-provenance.json').read_text());assert h['commit']==commit
for n,v in h['files'].items():assert hashlib.sha256((p/'harness'/n).read_bytes()).hexdigest()==v
print('ARCHIVED_F205_GATE_CUTAWAY_PACKAGE_AND_PUBLIC_SHORT_FLOW_PASS')
if '--artifacts'in sys.argv:
 import tarfile
 root=p.parents[2];d=root/meta['package_directory'];t=root/meta['tar'];sha=lambda x:hashlib.sha256(x.read_bytes()).hexdigest()
 assert sha(d/'rogue_commander')==meta['binary_sha256'] and sha(t)==meta['tar_sha256']
 rows=json.loads((p/'package/package-manifest.json').read_text())['files'];assert len(rows)==892
 for row in rows:assert sha(d/row['file'])==row['sha256'],row['file']
 with tarfile.open(t)as archive:
  files=[x for x in archive.getmembers()if x.isfile()];assert len(files)==893
  for x in files:
   n=pathlib.PurePosixPath(x.name).relative_to('rogue-commander-linux');assert archive.extractfile(x).read()==(d/str(n)).read_bytes() and x.uid==x.gid==x.mtime==0
 print('IMMUTABLE_F205_ARTIFACTS_PASS all892files and893tar members')
