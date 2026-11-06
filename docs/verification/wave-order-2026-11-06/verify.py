import gzip, hashlib, json, pathlib, re
p = pathlib.Path(__file__).resolve().parent
for n, h in json.loads((p/'verification.json').read_text())['files_sha256'].items():
    assert hashlib.sha256((p/n).read_bytes()).hexdigest() == h, n
before, after = [(p/n).read_text() for n in ['before-tests.log','after-focused.log']]
assert '0 passed; 2 failed' in before and '2 passed; 0 failed' in after
assert '704 passed; 0 failed; 1 ignored' in (p/'after-full.log').read_text()
assert (p/'before-exit.txt').read_text().strip() == '101'
assert (p/'clippy-exit.txt').read_text().strip() == '0'
assert 'TERMINAL AFTER PASS' in (p/'after-sequence.log').read_text()
for stage in ['before','after']:
    m = json.loads((p/(stage+'-inputs.json')).read_text())
    assert m == json.loads((p/(stage+'-inputs-after.json')).read_text())
    for path in ['src/game/skills.rs','src/game/skills/tidal_wave.rs','src/game/combat/wave_order_tests.rs']:
        n = pathlib.Path(path).name
        f = p/(stage+'-'+n)
        data = f.read_bytes() if f.exists() else gzip.decompress(pathlib.Path(str(f)+'.gz').read_bytes())
        assert hashlib.sha256(data).hexdigest() == m[path]
def outcomes(text):
    rows = [s for s in text.splitlines() if s.startswith('TIDAL_WAVE_ORDER ')]
    assert len(rows) == 4
    result = []
    for s in rows:
        assert 'damage=90.56 rng=16263334312487546840 casts=1 strikes=2 sweep_to=2.1000001 remaining=1.5642858 hit_memory=2' in s
        result.append(tuple(map(float, re.search(r'hp=\[([^]]+)\]',s)[1].split(','))))
    return result
assert set(outcomes(before)) == {(9909.44,14841.52),(9841.52,14909.44)}
assert set(outcomes(after)) == {(9909.44,14841.52)}
assert gzip.decompress((p/'before-skills.rs.gz').read_bytes()) == gzip.decompress((p/'after-skills.rs.gz').read_bytes())
b = gzip.decompress((p/'before-tidal_wave.rs.gz').read_bytes()).decode()
a = gzip.decompress((p/'after-tidal_wave.rs.gz').read_bytes()).decode()
start = a.index('            marked.push(target_info(target, t, enemy, boss, trial, archive));')
end = a.index('            wave.hit.insert(target);',start) + len('            wave.hit.insert(target);')
original = '            wave.hit.insert(target);\n            let info = target_info(target, t, enemy, boss, trial, archive);'
r = a[:start] + original + a[end:]
r = r.replace('            apply_control(&mut runtime, *info, 1.5, 0.0, 0.0);','            apply_control(&mut runtime, info, 1.5, 0.0, 0.0);\n            marked.push(info);',1)
assert r == b, 'only eligible target batch order and equivalent marked reuse may differ'
assert all(x['identical_to_237e650'] for x in json.loads((p/'source-comparison.json').read_text()).values())
print('PASS actual Wave 0/2→2/0 and704/0/1ignored, four HP/RNG/sweep-memory contracts, strict Clippy, frozen inputs and exact local longitudinal-scheduling delta')
