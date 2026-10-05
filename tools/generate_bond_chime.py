"""Generate the original ascending bond-completion chime (no external samples)."""
import math, struct, wave
from pathlib import Path
output = Path(__file__).resolve().parents[1] / "assets/audio/bond_complete.wav"
rate=44100
notes=[(0.0,523.25),(0.11,659.25),(0.22,783.99),(0.36,1046.50)]
frames=[]
for n in range(int(rate*1.6)):
 t=n/rate
 value=0.0
 for start,f in notes:
  age=t-start
  if age>=0:
   envelope=min(age/.008,1)*math.exp(-age*4.8)
   value+=envelope*(math.sin(2*math.pi*f*age)+.24*math.sin(2*math.pi*f*2*age)+.10*math.sin(2*math.pi*f*3*age))*.19
 frames.append(value)
peak=max(abs(v) for v in frames)
with wave.open(str(output),'wb') as out:
 out.setnchannels(1);out.setsampwidth(2);out.setframerate(rate)
 out.writeframes(b''.join(struct.pack('<h',int(v/max(1,peak)*28000)) for v in frames))
print(f'Generated original 1.6s completion chime; peak={peak:.3f}, bytes={output.stat().st_size}')
