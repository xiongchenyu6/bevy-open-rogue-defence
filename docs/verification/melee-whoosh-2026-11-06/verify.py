"""Offline verification of archived actual output; no game runs or source writes."""
from pathlib import Path
import array,hashlib,json,math,re,runpy,struct,wave
P=Path(__file__).resolve().parent
sha=lambda b:hashlib.sha256(b).hexdigest()
load=lambda p:json.loads(p.read_text())
assert '700 passed; 0 failed; 1 ignored' in (P/'final-full-tests.log').read_text()
assert 'Finished `release`' in (P/'final-clippy.log').read_text()
old=(P/'history/old-test.log').read_text();assert '0 passed; 1 failed' in old and 'left: 0' in old and 'right: 1' in old
assert '698 passed; 1 failed' in (P/'history/field-entity-allocation-behavior-failure.log').read_text()
a=load(P/'history/native-r4b/native-inputs-after.json');b=load(P/'history/native-r5b/native-inputs-after.json')
for version in ['native-r4b','native-r5b']:
 assert load(P/'history'/version/'native-inputs-before.json')==load(P/'history'/version/'native-inputs-after.json')
 assert 'Finished `release`' in (P/'history'/version/'clippy.log').read_text()
 assert 'Finished `dev`' in (P/'history'/version/'native-build.log').read_text()
for f in ['src/game/audio.rs','src/game/audio_melee.rs','src/game/combat.rs','src/game/hero_fusion.rs','src/game/combat/melee_audio_tests.rs','assets/audio/swing_sword.wav','assets/audio/swing_axe.wav','assets/audio/swing_daggers.wav']:
 assert a[f]==b[f]==sha((P/'source'/f).read_bytes()),f
# Confirm the current diagnostic-only basic_attack delta against the sword build.
diag='''/// Read-only capture diagnostic; never changes a pending attack.
#[cfg(any(debug_assertions, feature = "dev-tools"))]
pub fn pending_melee_count(world: &mut World) -> usize {
    world.query::<&PendingSwing>().iter(world).count()
}

'''
s=(P/'source/src/game/basic_attacks.rs').read_text();assert diag in s
assert sha(s.replace(diag,'',1).encode())==a['src/game/basic_attacks.rs']
originals=load(P/'original-files.json');correlations=load(P/'final-reference-correlations.json')
generator=runpy.run_path(str(P/'source/tools/generate_melee_whoosh.py'))
results={}
for kind in ['sword','axe','daggers']:
 case=P/'native'/kind;version='native-r4b' if kind=='sword' else 'native-r5b'
 process=load(case/'process.json');assert process['child_exit']==0
 text=(case/'game.log').read_text();assert 'panicked' not in text and 'PASS: real basic attack damage' in text and 'wrote 2000 simulation frames' in text
 assert 'audio stream error' not in text
 begin=re.search(r'MELEE_MISS_BEGIN frame=(\d+) real=([\d.]+) wall=([\d.]+) health=([\d.]+)',text)
 end=re.search(r'MELEE_MISS_END frame=(\d+) real=([\d.]+) wall=([\d.]+) health=([\d.]+)',text)
 assert begin and end and begin[4]==end[4]
 contacts=re.findall(r'MELEE_CONTACT frame=(\d+) real=([\d.]+).*?health=([\d.]+) damage=([\d.]+)',text)
 assert all(float(c[3])==0 and c[2]==begin[4] for c in contacts if int(begin[1])<=int(c[0])<=int(end[1]))
 first_hit=min(int(c[0]) for c in contacts if float(c[3])>0)
 voices=re.findall(r'MELEE_AUDIO frame=(\d+) real=([\d.]+).*?entity=(.*?) source=(.*?) sink=(true|false)',text)
 matching=[v for v in voices if ('audio/swing_'+kind+'.wav') in v[3] and int(v[0])<first_hit]
 assert matching and any(v[4]=='true' for v in matching)
 assert first_hit>int(end[1])
 if kind!='sword': assert 'pending=0' in re.search(r'MELEE_MISS_END[^\n]*',text)[0]
 with wave.open(str(case/'output.wav')) as w:
  assert (w.getnchannels(),w.getframerate(),w.getsampwidth())==(2,48000,2)
  full=w.readframes(w.getnframes())
 raw_path='target/verification/melee-whoosh-implementation-2026-11-06/'+version+'/native-'+kind+'/output.raw'
 assert sha(full)==originals[raw_path]['sha256']
 window=case/('first-miss-window.wav' if kind=='sword' else 'zero-damage-window.wav')
 with wave.open(str(window)) as w: pcm=array.array('h',w.readframes(w.getnframes()))
 assert len(pcm)>4800 and sum(v!=0 for v in pcm)>100
 with wave.open(str(P/'source/assets/audio'/('swing_'+kind+'.wav'))) as w:
  assert (w.getnchannels(),w.getframerate(),w.getsampwidth())==(1,48000,2)
  reference_bytes=w.readframes(w.getnframes());x=list(array.array('h',reference_bytes))
 assert max(map(abs,x))<32767
 generated=generator['generate'](generator['PARAMS'][kind])
 assert struct.pack('<'+'h'*len(generated),*(round(v*32767) for v in generated))==reference_bytes
 y=[(pcm[i]+pcm[i+1])/2 for i in range(0,len(pcm),2)]
 onset=next(i for i,v in enumerate(y) if abs(v)>16);ref_onset=next(i for i,v in enumerate(x) if abs(v)>64)
 xx=sum(v*v for v in x);best=(-1,None,None)
 for offset in range(max(0,onset-ref_onset-300),min(len(y)-len(x),onset-ref_onset+300)+1):
  segment=y[offset:offset+len(x)];yy=sum(v*v for v in segment)
  if yy:
   xy=sum(v*w for v,w in zip(x,segment));c=xy/math.sqrt(xx*yy)
   if c>best[0]:best=(c,offset,xy/xx)
 r=correlations[kind];assert abs(best[0]-r['normalized_cross_correlation'])<1e-12 and abs(best[2]-r['least_squares_gain'])<1e-12
 results[kind]=dict(child_exit=0,first_source_frame=min(int(v[0]) for v in matching),first_hp_frame=first_hit,correlation=best[0],gain=best[2])
assert 'MELEE_WHOOSH_NATIVE_PASS' in (P/'history/native-r5b/manager.log').read_text()
# Historical failed managers are retained; do not silently upgrade them.
assert 'AssertionError: no recorded energy' in (P/'history/native-r4b/manager.log').read_text()
assert load(P/'history/native-r3/native-sword/process.json')['child_exit']==101
if (P/'files.sha256.json').exists():
 for f,h in load(P/'files.sha256.json').items():assert sha((P/f).read_bytes())==h,f
print('MELEE_WHOOSH_ARCHIVE_VERIFIED '+json.dumps(dict(library_pass=700,library_ignored=1,native=results,listening_verified=False)))
