from pathlib import Path
import hashlib,json,shutil,subprocess
ROOT=Path.cwd();SRC=ROOT/'target/verification/melee-whoosh-implementation-2026-11-06';DST=ROOT/'docs/verification/melee-whoosh-2026-11-06'
assert not DST.exists();DST.mkdir(parents=True)
# Retain every raw text log/manager/receipt, including all failed/interrupted stages.
for p in sorted(SRC.rglob('*')):
 if p.is_file() and p.suffix in ['.log','.json','.py','.conf','.pa']:
  q=DST/'history'/p.relative_to(SRC);q.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,q)
files=['src/game/audio.rs','src/game/audio_melee.rs','src/game/basic_attacks.rs','src/game/combat.rs','src/game/combat/melee_audio_tests.rs','src/game/hero_fusion.rs','src/game/mod.rs','src/bin/capture/attacks.rs','tools/generate_melee_whoosh.py','assets/audio/SOURCES.md','assets/audio/swing_sword.wav','assets/audio/swing_axe.wav','assets/audio/swing_daggers.wav']
for f in files:
 q=DST/'source'/f;q.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(ROOT/f,q)
casepaths={k:SRC/('native-r4b' if k=='sword' else 'native-r5b')/('native-'+k) for k in ['sword','axe','daggers']}
for k,p in casepaths.items():
 q=DST/'native'/k;q.mkdir(parents=True)
 for f in p.rglob('*'):
  if f.is_file() and f.suffix in ['.log','.json','.wav','.png']:
   dest=q/f.relative_to(p);dest.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(f,dest)
for f in ['final-full-tests.log','final-clippy.log','runtime-library-final-hashes.json','actual-test-boundaries.json','final-reference-correlations.json','correlate-final.py']:
 shutil.copy2(SRC/f,DST/f)
# Preserve the candidate plots and numerical generation measurements.
candidates=ROOT/'target/verification/melee-whoosh-candidates-2026-11-06'
for f in ['measurements.json','waveform-spectrum.png','waveform-spectrum.svg','generate.py']:
 q=DST/'candidates'/f;q.parent.mkdir(exist_ok=True);shutil.copy2(candidates/f,q)
originals={str(p.relative_to(ROOT)):dict(sha256=hashlib.sha256(p.read_bytes()).hexdigest(),bytes=p.stat().st_size) for p in sorted(SRC.rglob('*')) if p.is_file()}
(DST/'original-files.json').write_text(json.dumps(originals,indent=2)+'\n')
(DST/'provenance.json').write_text(json.dumps(dict(base_head=subprocess.check_output(['git','rev-parse','HEAD'],text=True).strip(),source_scope='Base HEAD plus scoped audio/diagnostic changes, exact copied source and native receipts; no standalone release package or public campaign rerun',native_cases={k:str(p.relative_to(ROOT)) for k,p in casepaths.items()},library_test_scope='700 pass/0 fail/1 ignored on final runtime; later only cfg readonly count/reexport and controlled capture diagnostics changed; exact audio/core hashes retained',listening_verified=False),indent=2)+'\n')
(DST/'.gitattributes').write_text('*.log -text\n*.wav -text\n*.png -text\n')
print(DST)
