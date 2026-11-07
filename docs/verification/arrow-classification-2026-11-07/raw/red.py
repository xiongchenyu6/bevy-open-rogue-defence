from pathlib import Path
import subprocess,json,hashlib
r=Path.cwd();p=r/'target/verification/arrow-classification-2026-11-07'
files=[f for f in r.glob('src/**/*.rs')]+[r/'Cargo.toml',r/'Cargo.lock'];a={str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in files}
(p/'red-inputs.json').write_text(json.dumps(a,indent=2))
for f in ['src/game/combat.rs','src/game/combat/arrow_classification_tests.rs','src/game/progression.rs']:
 (p/('red-'+Path(f).name)).write_bytes((r/f).read_bytes())
cmd=['nix','develop','--command','cargo','test','--locked','--release','--lib','arrow_classification_native_ember','-j2','--','--nocapture']
with (p/'red.log').open('w') as log:x=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
(p/'red-result.json').write_text(json.dumps({'command':cmd,'returncode':x.returncode}))
assert a=={str(f.relative_to(r)):hashlib.sha256(f.read_bytes()).hexdigest() for f in files}
text=(p/'red.log').read_text();assert x.returncode!=0 and 'ARROW_NATIVE' in text and 'actual native HP losses' in text
print('EXPECTED REAL HP NEGATIVE VERIFIED',flush=True)
