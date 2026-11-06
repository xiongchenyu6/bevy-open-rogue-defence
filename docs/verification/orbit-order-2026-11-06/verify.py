import gzip, hashlib, json, pathlib, re
p = pathlib.Path(__file__).resolve().parent
for n, h in json.loads((p/'verification.json').read_text())['files_sha256'].items():
    assert hashlib.sha256((p/n).read_bytes()).hexdigest() == h, n
before = (p/'before-tests.log').read_text()
after = (p/'after-focused.log').read_text()
assert '0 passed; 2 failed' in before and '2 passed; 0 failed' in after
assert '702 passed; 0 failed; 1 ignored' in (p/'after-full.log').read_text()
assert (p/'before-exit.txt').read_text().strip() == '101'
assert (p/'clippy-exit.txt').read_text().strip() == '0'
assert 'TERMINAL AFTER PASS' in (p/'after-sequence.log').read_text()
for stage in ['before','after']:
    assert json.loads((p/(stage+'-inputs.json')).read_text()) == json.loads((p/(stage+'-inputs-after.json')).read_text())
def outcomes(text):
    rows = [s for s in text.splitlines() if s.startswith('ORBIT_BLADE_ORDER ')]
    assert len(rows) == 4
    result = []
    for s in rows:
        assert 'damage=44.8 rng=16263334312487546840 strikes=2' in s
        assert 'cooldowns=[(37v0, 1.0), (36v0, 1.0)]' in s
        assert 'blades=[Vec3(2.05, 0.48, 0.0), Vec3(-2.05, 0.48, -1.7921668e-7)]' in s
        result.append(tuple(map(float, re.search(r'hp=\[([^]]+)\]', s)[1].split(','))))
    return result
assert set(outcomes(before)) == {(9955.2,14921.6),(9921.6,14955.2)}
assert set(outcomes(after)) == {(9955.2,14921.6)}
b = gzip.decompress((p/'before-skills.rs.gz').read_bytes()).decode()
a = gzip.decompress((p/'after-skills.rs.gz').read_bytes()).decode()
for stage, text in [('before',b),('after',a)]:
    assert hashlib.sha256(text.encode()).hexdigest() == json.loads((p/(stage+'-inputs.json')).read_text())['src/game/skills.rs']
# Independently delimit the only production addition between the tested files.
r = a.replace('        let mut contacts = Vec::new();\n', '', 1)
start = r.index('            contacts.push((enemy_entity, enemy_transform.translation));')
end = r.index('        for (enemy_entity, _) in contacts {',start) + len('        for (enemy_entity, _) in contacts {\n')
assert r[:start] + r[end:] == b
for source, change in json.loads((p/'comment-only-final-delta.json').read_text()).items():
    n = pathlib.Path(source).name
    def read(stage):
        f = p/(stage+'-'+n)
        return gzip.decompress(pathlib.Path(str(f)+'.gz').read_bytes()).decode() if n == 'skills.rs' else f.read_text()
    tested, final = read('after'), read('final')
    assert hashlib.sha256(tested.encode()).hexdigest() == change['tested_sha256']
    assert hashlib.sha256(final.encode()).hexdigest() == change['final_sha256']
    old,new = change['exact_comment_replacement'].values()
    assert tested.count(old) == 1 and tested.replace(old,new) == final
assert all(x['identical_to_237e650'] for x in json.loads((p/'source-comparison.json').read_text()).values())
print('PASS actual Orbit 0/2→2/0 and702/0/1ignored, four HP/RNG/contact contracts, strict Clippy, frozen inputs, limited production addition and explicit final comment-only SHA delta')
