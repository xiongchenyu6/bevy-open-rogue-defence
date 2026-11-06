"""Original procedural candidates; stdlib only, no external samples/services.
Primitives follow tools/generate_hit_audio.py: seeded noise, one-pole LP/HP,
envelope and PCM16 wave output. New envelopes/frequency sweeps below.
Run: python3 target/verification/melee-whoosh-candidates-2026-11-06/generate.py
"""
import math, random, struct, wave, json, hashlib
from pathlib import Path
ROOT=Path(__file__).resolve().parent
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
def rms(s): return math.sqrt(sum(x*x for x in s)/len(s)) if s else 0
report={}
svg=['<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="900" viewBox="0 0 1200 900"><rect width="1200" height="900" fill="#101923"/><style>text{font:15px sans-serif;fill:#e3edf4} .axis{stroke:#677889;stroke-width:1}</style>']
for row,(name,p) in enumerate(PARAMS.items()):
 s=generate(p); path=ROOT/(name+'.wav')
 with wave.open(str(path),'wb') as w:
  w.setparams((1,2,RATE,0,'NONE','not compressed')); w.writeframes(struct.pack('<'+'h'*len(s),*(round(x*32767) for x in s)))
 with wave.open(str(path)) as w:
  assert (w.getnchannels(),w.getsampwidth(),w.getframerate())==(1,2,RATE)
  raw=w.readframes(w.getnframes()); decoded=[x/32768 for x in struct.unpack('<'+'h'*(len(raw)//2),raw)]
 windows=[rms(decoded[i:i+240]) for i in range(0,len(decoded),240)]
 peak_i=max(range(len(windows)),key=windows.__getitem__)
 assert max(map(abs,decoded))<.99 and peak_i>0 and peak_i<len(windows)-1
 assert windows[0]<windows[peak_i]*.3 and windows[-1]<windows[peak_i]*.1
 report[name]=dict(parameters=p,seconds=len(decoded)/RATE,rate=RATE,channels=1,bits=16,peak=max(map(abs,decoded)),rms=rms(decoded),rms_peak_window_start_seconds=peak_i*.005,first_5ms_rms=windows[0],peak_5ms_rms=windows[peak_i],last_5ms_rms=windows[-1],clipped_samples=sum(abs(x)>=.999 for x in decoded),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),decoded=True)
 y=row*300
 svg.append(f'<text x="30" y="{y+25}">{name}: {p["seconds"]*1000:.0f} ms · peak {report[name]["peak"]:.3f} · RMS {rms(decoded):.3f}</text>')
 svg.append(f'<text x="30" y="{y+52}">Waveform (seconds)</text><text x="640" y="{y+52}">Whole-cue spectrum (0–12 kHz, dB relative peak)</text>')
 # Peak-preserving min/max waveform, no downsample aliasing.
 for x in range(520):
  block=decoded[int(x*len(decoded)/520):max(int((x+1)*len(decoded)/520),int(x*len(decoded)/520)+1)]
  svg.append(f'<line x1="{30+x}" x2="{30+x}" y1="{y+155-95*max(block)}" y2="{y+155-95*min(block)}" stroke="#5bdac3"/>')
 svg.append(f'<path class="axis" d="M30 {y+155}H550 M30 {y+65}V{y+250}"/><text x="30" y="{y+275}">0</text><text x="490" y="{y+275}">{p["seconds"]:.3f}s</text>')
 # Exact full-signal spectral samples, Hann-windowed; no scipy/numpy dependency.
 hann=[v*(.5-.5*math.cos(2*math.pi*i/(len(decoded)-1))) for i,v in enumerate(decoded)]
 powers=[]
 for k in range(1,241):
  f=k*50; step=2*math.pi*f/RATE
  re=sum(v*math.cos(step*i) for i,v in enumerate(hann)); im=sum(v*math.sin(step*i) for i,v in enumerate(hann))
  powers.append(math.hypot(re,im))
 top=max(powers); points=' '.join(f'{640+i*2.1:.1f},{y+80+min(60,-20*math.log10(max(v/top,1e-6)))*2.6:.1f}' for i,v in enumerate(powers))
 svg.append(f'<polyline points="{points}" fill="none" stroke="#ffc675"/><path class="axis" d="M640 {y+236}H1150"/><text x="640" y="{y+275}">0 kHz</text><text x="1090" y="{y+275}">12 kHz</text>')
svg.append('</svg>'); (ROOT/'waveform-spectrum.svg').write_text(''.join(svg))
(ROOT/'measurements.json').write_text(json.dumps(dict(source='Original deterministic synthesis; no external samples. Primitives referenced from repository tools/generate_hit_audio.py; new parameters and smooth envelope in this script.',listening_verified=False,candidates=report),indent=2)+'\n')
print(json.dumps(report,indent=2))
