import pathlib,json,hashlib,subprocess,sys
p=pathlib.Path(__file__).resolve().parent;m=json.loads((p/'verification.json').read_text())
for n,w in m['files_sha256'].items():assert hashlib.sha256((p/n).read_bytes()).hexdigest()==w,n
for a,b in [('after-inputs.json','after-inputs-after.json'),('release-inputs-before.json','release-inputs-after.json'),('growth/inputs-before.json','growth/inputs-after.json')]:assert json.loads((p/a).read_text())==json.loads((p/b).read_text())
native=json.loads((p/'after-inputs-after.json').read_text());release=json.loads((p/'release-inputs-after.json').read_text());growth=json.loads((p/'growth/inputs-after.json').read_text())
assert [n for n in native if native[n]!=release[n]]==['src/game/basic_attacks.rs']
assert [n for n in release if release[n]!=growth[n]]==['src/bin/capture/attacks.rs']
old=(p/'native-candidate-basic-attacks.rs').read_text().split('#[cfg(test)]')[0]
assert old==(p/'verified-basic-attacks.rs').read_text().split('#[cfg(test)]')[0]
assert hashlib.sha256((p/'verified-basic-attacks.rs').read_bytes()).hexdigest()==release['src/game/basic_attacks.rs']
assert hashlib.sha256((p/'growth/verified-capture-attacks.rs').read_bytes()).hexdigest()==growth['src/bin/capture/attacks.rs']
assert hashlib.sha256(old.encode()).hexdigest()==json.loads((p/'test-only-extension.json').read_text())['native_production_sha256']
assert '689 passed; 0 failed; 1 ignored' in (p/'full-tests.log').read_text()
assert 'TERMINAL RELEASE PASS' in (p/'release-sequence.log').read_text()
assert 'TERMINAL GROWTH PASS' in (p/'growth/sequence.log').read_text()
subprocess.run([sys.executable,str(p/'compare-void-contacts.py')],check=True)
subprocess.run([sys.executable,str(p/'compare.py')],check=True)
r=json.loads((p/'growth/growth-audit.json').read_text())
for kind,layers in [('void',2),('daggers',3)]:
 assert r[kind]['actual_level40_fixture_passed'] and r[kind]['peak_simultaneous_visible_trails']>=layers
 assert r[kind]['first_trail_disappears_while_later_trails_continue']
print('PASS staged source/hash/native689test proof; bounded melee art only')
