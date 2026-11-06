import gzip, hashlib, json, pathlib, re
p = pathlib.Path(__file__).resolve().parent
m = json.loads((p/'verification.json').read_text())
for name, expected in m['files_sha256'].items():
    assert hashlib.sha256((p/name).read_bytes()).hexdigest() == expected, name
assert 'error[E0277]' in (p/'r1-compile-invalid-before-tests.log').read_text()
assert '0 passed; 1 failed' in (p/'before-tests.log').read_text()
assert 'first_scale=Vec3(1.25, 1.25, 1.25) active=false' in (p/'before-tests.log').read_text()
assert '693 passed; 0 failed; 1 ignored' in (p/'after-tests.log').read_text()
assert (p/'clippy-exit.txt').read_text().strip() == '0'
assert 'TERMINAL AFTER PASS' in (p/'after-sequence.log').read_text()
a = json.loads((p/'after-inputs.json').read_text())
assert a == json.loads((p/'after-inputs-after.json').read_text())
b = json.loads((p/'before-inputs.json').read_text())
for source in ['src/game/hit_feedback.rs', 'src/game/hit_feedback/enemy_flash.rs', 'src/bin/capture/hit_material.rs']:
    name = pathlib.Path(source).name
    assert hashlib.sha256(gzip.decompress((p/('before-'+name+'.gz')).read_bytes())).hexdigest() == b[source]
    assert hashlib.sha256(gzip.decompress((p/('after-'+name+'.gz')).read_bytes())).hexdigest() == a[source]
texts = [gzip.decompress((p/(stage+'-native.log.gz')).read_bytes()).decode() for stage in ['before','after']]
for text, visible in zip(texts, ['false','true']):
    assert f'visible={visible} real_delta=0.16' in text
    assert 'HIT_MATERIAL_PASS' in text and 'PASS: real basic attack damage' in text
    assert 'panicked' not in text
contacts = []
for text in texts:
    contacts.append(sorted((int(f), float(h), float(d)) for f,h,d in re.findall(r'HIT_MATERIAL_CONTACT frame=(\d+) actor=\S+ hp=([\d.]+) damage=([\d.]+)', text)))
assert len(contacts[0]) == len(contacts[1]) == 255 and contacts[0] == contacts[1]
timing = [[s for s in text.splitlines() if s.startswith('ATTACK_TIMING ')] for text in texts]
assert len(timing[0]) == len(timing[1]) == 80 and timing[0] == timing[1]
assert json.loads((p/'comparison.json').read_text())['rng_telemetry'] is False
print('PASS first-contact actual native scale, 255 contacts/80 timing rows equal, 693 tests, strict Clippy, frozen input hashes and archive integrity; RNG unmeasured')
