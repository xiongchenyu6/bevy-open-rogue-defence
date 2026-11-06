"""Draft integrity/scope check; --require-smoke needs actual final strict smoke0."""
import hashlib,json,pathlib,re,struct,sys
D=pathlib.Path(__file__).resolve().parent
ROOT=D.parents[2]
for name,value in json.loads((D/'SHA256.json').read_text()).items():assert hashlib.sha256((D/name).read_bytes()).hexdigest()==value,name
for f in json.loads((D/'archive-files.json').read_text()):
    p=D/f['archive'];assert p.stat().st_size==f['bytes'] and hashlib.sha256(p.read_bytes()).hexdigest()==f['sha256'],p
for case in ('r2-gpu','r3-strict24'):
    assert json.loads((D/'raw'/case/'smoke-process.json').read_text())['exit']==1
for case in ('public-r3-unknown','public-r4-camp','public-r5-menu'):
    assert json.loads((D/'raw'/case/'process.json').read_text())['exit']==1
    assert json.loads((D/'raw'/case/'output-result.json').read_text())['completed'] is False
assert 'Attachments have differing sizes' in (D/'raw/r2-gpu/smoke-game.log').read_text()
assert '24\\s*:' in (D/'raw/r3-strict24/smoke.log').read_text()
assert json.loads((D/'raw/strict-smoke-r2-settings/process.json').read_text())['exit']==1
assert 'Timed out waiting for runtime UI' in (D/'raw/strict-smoke-r2-settings/runtime.log').read_text()
assert not re.search(r'\bERROR\b',(D/'raw/strict-smoke-r2-settings/output-game.log').read_text())
assert json.loads((D/'manual/exit.json').read_text())['game_exit']==0
unit=(D/'manual/unit-show.txt').read_text();assert 'Result=timeout' in unit and 'ExecMainStatus=0' in unit
assert 'runtime time limit' in (D/'manual/unit-journal.log').read_text()
assert not re.search(r'\bERROR\b', (D/'manual/game.log').read_text())
rows=[json.loads(x) for x in (D/'manual/actions.jsonl').read_text().splitlines()]
geometry=[r['stdout'] for r in rows if r['args'][1:2]==['getwindowgeometry'] and r['exit']==0]
for w,h in [(375,667),(1600,1000),(1280,720)]:assert any(f'WIDTH={w}\nHEIGHT={h}\n' in g for g in geometry),(w,h)
assert sum(r['args'][1:]==['keydown','F11'] for r in rows)==4
for name,size in [('portrait-settings.png',(375,667)),('fullscreen-settings.png',(1600,1000)),('portrait-restored-settings.png',(375,667)),('desktop-restored.png',(1280,720)),('quit-confirmation.png',(1280,720))]:
    p=D/'manual'/name;assert struct.unpack('>II',p.read_bytes()[16:24])==size
status=json.loads((D/'full-smoke-status.json').read_text())
assert status['status']=='ACTUAL_TERMINAL_EXIT0_FULL_FLOW'
smoke=D/status['archived']
process=json.loads((smoke/'process.json').read_text());assert process['exit']==0
positive_unit=(smoke/'unit-show.txt').read_text();assert 'Result=success' in positive_unit and 'ExecMainStatus=0' in positive_unit
result=json.loads((smoke/'output/result.json').read_text());assert result['normal_quit_twice'] is True
assert all(v for v in result.values() if isinstance(v,bool))
for name in ('initial','restart'):
    proof=result['battle_clock_proofs'][name]
    assert proof['second']['clock_seconds']<proof['first']['clock_seconds']
provenance=json.loads((smoke/'harness-provenance.json').read_text())
assert provenance['harness_commit'].startswith('c2a0740')
assert provenance['package_commit']=='932fba4d3f4a172e9c7421be1b191bb6b989127e'
for name,value in provenance['files'].items():assert hashlib.sha256((smoke/'harness'/name).read_bytes()).hexdigest()==value
for name in ('game.log','restart.log'):assert not re.search(r'\bERROR\b',(smoke/'output'/name).read_text())
print('ARCHIVED_CAVE_FEEL_SDK_AND_STRICT_SMOKE_PASS; six negative attempts retained; manual unit TIMEOUT/game exit0')

if '--artifacts' in sys.argv:
 import tarfile
 a=json.loads((D/'package/artifact.json').read_text());p=ROOT/a['package_directory'];t=ROOT/a['tar']
 sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
 assert sha(p/'rogue_commander')==a['binary_sha256'] and sha(t)==a['tar_sha256']
 m=json.loads((D/'package/package-manifest.json').read_text())['files'];assert len(m)==892
 for row in m:assert sha(p/row['file'])==row['sha256'],row['file']
 with tarfile.open(t)as archive:
  rows=[row for row in archive.getmembers()if row.isfile()];assert len(rows)==893
  for row in rows:
   n=pathlib.PurePosixPath(row.name).relative_to('rogue-commander-linux')
   assert archive.extractfile(row).read()==(p/str(n)).read_bytes() and row.uid==row.gid==row.mtime==0
 print('IMMUTABLE_932_ARTIFACTS_PASS all892files and893tar members')
