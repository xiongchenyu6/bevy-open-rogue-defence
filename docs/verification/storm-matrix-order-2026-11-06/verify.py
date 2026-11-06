import gzip, hashlib, json, pathlib, re
p = pathlib.Path(__file__).resolve().parent
m = json.loads((p / 'verification.json').read_text())
for n, h in m['files_sha256'].items():
    assert hashlib.sha256((p / n).read_bytes()).hexdigest() == h, n
before = (p / 'before-tests.log').read_text()
after = (p / 'after-tests.log').read_text()
assert '1 passed; 1 failed' in before
assert '691 passed; 0 failed; 1 ignored' in after
assert 'clippy terminal 0' in (p / 'after-sequence.log').read_text()
assert 'source hashes unchanged' in (p / 'after-pass.txt').read_text()
assert (p / 'before-exit.txt').read_text().strip() == '101'
assert (p / 'clippy-exit.txt').read_text().strip() == '0'
assert 'left: 0' in (p / 'r1-fixture-invalid.log').read_text()
def outcomes(text):
    rows = []
    for line in text.splitlines():
        if line.startswith('STORM_MATRIX_ORDER '):
            hp = tuple(float(x) for x in re.search(r'hp=\[([^]]+)\]', line)[1].split(','))
            assert 'damage=49.2296 rng=16263334312487546840 casts=1 strikes=2 pulse=0.6 remaining=3.4' in line
            rows.append(hp)
    assert len(rows) == 4
    return rows
assert set(outcomes(before)) == {(9913.849, 14950.7705), (9950.7705, 14913.849)}
assert set(outcomes(after)) == {(9950.7705, 14913.849)}
b = gzip.decompress((p / 'before-skills.rs.gz').read_bytes())
a = gzip.decompress((p / 'after-skills.rs.gz').read_bytes())
assert hashlib.sha256(b).hexdigest() == json.loads((p / 'before-source-hashes.json').read_text())['src/game/skills.rs']
assert hashlib.sha256(a).hexdigest() == json.loads((p / 'after-source-hashes.json').read_text())['src/game/skills.rs']
start = a.index(b'        // Storm pulses bind shared critical rolls')
end = a.index(b'\n\n        for _ in 0..pulses', start)
original = b'''        // Cosmetic component migrations can reorder the enemy query. Keep
        // target-to-strike assignment stable before shared critical RNG is used.
        affected.sort_unstable_by_key(|target| target.entity.to_bits());'''
assert a[:start] + original + a[end:] == b
print('PASS actual before 1/1, after 691/0/1ignored, strict Clippy, four HP/RNG outcomes, exact limited production delta and archive hashes')
