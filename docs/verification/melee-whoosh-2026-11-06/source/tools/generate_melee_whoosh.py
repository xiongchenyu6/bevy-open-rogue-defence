"""Original procedural candidates; stdlib only, no external samples/services.
Primitives follow tools/generate_hit_audio.py: seeded noise, one-pole LP/HP,
envelope and PCM16 wave output. New envelopes/frequency sweeps below.
Run: python3 tools/generate_melee_whoosh.py
"""
import math, random, struct, wave
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]/"assets/audio"
RATE=48000
PARAMS={
 'sword':dict(seconds=.19,seed=731,peak_time=.055,low_start=480,low_end=1800,high_start=4300,high_end=2200,gain=.53),
 'axe':dict(seconds=.25,seed=129,peak_time=.085,low_start=140,low_end=650,high_start=2200,high_end=1000,gain=.60),
 'daggers':dict(seconds=.115,seed=953,peak_time=.026,low_start=1100,low_end=2700,high_start=7600,high_end=3900,gain=.43),
}
def generate(p):
 rng=random.Random(p['seed']); a=b=0.; out=[]
 for i in range(round(p['seconds']*RATE)):
  t=i/RATE; u=t/p['seconds']; low=p['low_start']+(p['low_end']-p['low_start'])*u
  high=p['high_start']+(p['high_end']-p['high_start'])*u
  n=rng.uniform(-1,1)
  a+=(1-math.exp(-2*math.pi*high/RATE))*(n-a)
  b+=(1-math.exp(-2*math.pi*low/RATE))*(a-b)
  # Smooth zero-ended rise and release; no contact transient or tonal impact.
  if t<p['peak_time']: env=math.sin(math.pi*.5*t/p['peak_time'])**1.6
  else: env=math.cos(math.pi*.5*(t-p['peak_time'])/(p['seconds']-p['peak_time']))**2.5
  out.append((a-b)*env)
 peak=max(map(abs,out)); return [x*p['gain']/peak for x in out]
if __name__ == "__main__":
 for name,p in PARAMS.items():
  samples=generate(p)
  path=ROOT/("swing_"+name+".wav")
  with wave.open(str(path),'wb') as w:
   w.setparams((1,2,RATE,0,'NONE','not compressed'))
   w.writeframes(struct.pack('<'+'h'*len(samples),*(round(x*32767) for x in samples)))
  print(path)
