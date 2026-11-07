import pathlib,subprocess,json,hashlib,shutil
r=pathlib.Path.cwd();p=r/'target/verification/ranged-release-2026-11-06'
a={str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in [*r.glob('src/**/*.rs'),*r.glob('assets/**/*'),r/'Cargo.toml',r/'Cargo.lock'] if f.is_file()};(p/'before-inputs.json').write_text(json.dumps(a,indent=2))
for name in ['combat.rs','combat/ranged_release_tests.rs']:
 dest=p/('before-'+name.replace('/','-'));shutil.copy2(r/'src/game'/name,dest)
cmd=['nix','develop','--command','cargo','test','--locked','--release','--lib','native_ranged_does_not_emit_at_attack_commit','-j2','--','--nocapture']
with (p/'before-test.log').open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
(p/'before-result.json').write_text(json.dumps({'command':cmd,'returncode':x.returncode}))
assert x.returncode==101
text=(p/'before-test.log').read_text();assert '0 passed; 1 failed' in text and 'real native projectile emitted before candidate phase' in text
assert all(hashlib.sha256((r/f).read_bytes()).hexdigest()==h for f,h in a.items())
print('BEFORE actual production native emit negative verified',flush=True)
