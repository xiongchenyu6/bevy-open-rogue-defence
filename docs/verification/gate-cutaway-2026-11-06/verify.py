import pathlib,json,hashlib,re
p=pathlib.Path(__file__).resolve().parent
for name,value in json.loads((p/'SHA256.json').read_text()).items():assert hashlib.sha256((p/name).read_bytes()).hexdigest()==value,name
assert '734 passed; 0 failed; 1 ignored'in(p/'checks/full-tests.log').read_text()
assert all(x['exit']==0 for x in json.loads((p/'checks/processes.json').read_text()))
r=json.loads((p/'native/results.json').read_text());assert len(r)==6 and sum(x['frames']for x in r)==3480 and all(x['exit']==0 for x in r)
traces={}
for row in r:
 log=(p/'native'/row['case']/'game.log').read_text();assert 'ERROR'not in log and 'panicked'not in log
 assert '[capture] PASS gate-cutaway:'in log
 lines=[x for x in log.splitlines()if '[capture] GATE_CUTAWAY 'in x];assert len(lines)==9
 traces[row['case']]=[re.search(r'frame=(\d+).*hero=(.*)',x).groups()for x in lines]
 for x in lines:
  fade='blocked=true'in x;assert f'opacity={0.1 if fade else 1.:.6f}'in x
for label in ('en-desktop','zh-compact','en-phone'):assert traces[label+'-gate-cutaway']==traces[label+'-gate-cutaway-opaque'],label
assert json.loads((p/'initial-capture-compile-failure/processes.json').read_text())[-1]['exit']==101
print('ARCHIVED_GATE_CUTAWAY_NATIVE_PASS; controlled original-material comparison, not public input or a new run')
