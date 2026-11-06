"""Offline evidence only. Same fixed R3 algorithm; no pass threshold tuning."""
from pathlib import Path
import wave,array,math,json
ROOT=Path.cwd();OUT=ROOT/'target/verification/melee-whoosh-implementation-2026-11-06/native-r4b'
result={}
for kind in ['sword','axe','daggers']:
 p=OUT/('native-'+kind)
 if not (p/'first-miss-window.wav').exists():
  result[kind]=dict(status='no usable exported zero-hit window' if (p/'process.json').exists() else 'not executed',normalized_cross_correlation=None,least_squares_gain=None,listening_verified=False)
  continue
 with wave.open(str(p/'first-miss-window.wav')) as w:
  assert (w.getnchannels(),w.getframerate(),w.getsampwidth())==(2,48000,2)
  pcm=array.array('h',w.readframes(w.getnframes()))
 y=[(pcm[i]+pcm[i+1])/2 for i in range(0,len(pcm),2)]
 with wave.open(str(ROOT/('assets/audio/swing_'+kind+'.wav'))) as w:x=list(array.array('h',w.readframes(w.getnframes())))
 onset=next((i for i,v in enumerate(y) if abs(v)>16),None);reference_onset=next(i for i,v in enumerate(x) if abs(v)>64)
 best=(-1,None,None)
 if onset is not None:
  estimate=onset-reference_onset;xx=sum(v*v for v in x)
  for offset in range(max(0,estimate-300),min(len(y)-len(x),estimate+300)+1):
   segment=y[offset:offset+len(x)];yy=sum(v*v for v in segment)
   if yy:
    xy=sum(v*w for v,w in zip(x,segment));c=xy/math.sqrt(xx*yy)
    if c>best[0]:best=(c,offset,xy/xx)
 r=dict(normalized_cross_correlation=best[0],offset_samples=best[1],offset_seconds=best[1]/48000 if best[1] is not None else None,least_squares_gain=best[2],reference='assets/audio/swing_'+kind+'.wav',search='independent PCM threshold onset neighborhood +/-300 samples; output threshold16/reference64; mono average; 48k no resampling',listening_verified=False,alignment='Native window from record end minus duration is approximate; not sample-accurate event synchronization',scope='Controlled Automatic 1APS/max_delta50ms first-miss zero-HP/music0 interval; no population/performance claim')
 (p/'reference-correlation.json').write_text(json.dumps(r,indent=2)+'\n');result[kind]=r
(OUT/'reference-correlations.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
