"""Generate original combat-feel sound effects (no external samples).

Every cue layers a sharp transient, a filtered noise burst and a pitched body
whose frequency drops quickly. That combination reads as "impact" even at low
volume; the runtime adds pitch jitter so repeated hits do not machine-gun.
"""
import math
import random
import struct
import wave
from pathlib import Path

RATE = 48000
AUDIO_DIR = Path(__file__).resolve().parents[1] / "assets/audio"


def one_pole_lowpass(samples, cutoff):
    alpha = 1.0 - math.exp(-2.0 * math.pi * cutoff / RATE)
    out, state = [], 0.0
    for value in samples:
        state += alpha * (value - state)
        out.append(state)
    return out


def highpass(samples, cutoff):
    low = one_pole_lowpass(samples, cutoff)
    return [value - smooth for value, smooth in zip(samples, low)]


def noise(seconds, seed):
    rng = random.Random(seed)
    return [rng.uniform(-1.0, 1.0) for _ in range(int(RATE * seconds))]


def sweep(seconds, start_hz, end_hz, drop_rate, decay, harmonics=((1.0, 1.0),)):
    """Exponentially gliding oscillator with an exponential amplitude decay."""
    out, phase = [], 0.0
    for n in range(int(RATE * seconds)):
        t = n / RATE
        freq = end_hz + (start_hz - end_hz) * math.exp(-drop_rate * t)
        phase += 2.0 * math.pi * freq / RATE
        value = sum(gain * math.sin(phase * mult) for mult, gain in harmonics)
        out.append(value * math.exp(-decay * t))
    return out


def envelope(samples, attack, decay):
    return [
        value * min(1.0, (n / RATE) / attack) * math.exp(-decay * n / RATE)
        for n, value in enumerate(samples)
    ]


def mix(*layers):
    length = max(len(layer) for layer, _ in layers)
    out = [0.0] * length
    for layer, gain in layers:
        for n, value in enumerate(layer):
            out[n] += value * gain
    return out


def soft_clip(samples, drive):
    return [math.tanh(value * drive) / math.tanh(drive) for value in samples]


def write(name, samples, peak_level=0.89):
    tail = int(RATE * 0.004)
    for n in range(min(tail, len(samples))):
        samples[-1 - n] *= n / tail
    peak = max(abs(value) for value in samples) or 1.0
    path = AUDIO_DIR / name
    with wave.open(str(path), "wb") as out:
        out.setnchannels(1)
        out.setsampwidth(2)
        out.setframerate(RATE)
        out.writeframes(
            b"".join(
                struct.pack("<h", int(value / peak * peak_level * 32767))
                for value in samples
            )
        )
    print(f"{name}: {len(samples) / RATE:.3f}s, {path.stat().st_size} bytes")


def hit_physical():
    # Meaty slap: click + mid noise crack + low thump.
    click = envelope(highpass(noise(0.012, 1), 2500), 0.0004, 380)
    crack = envelope(one_pole_lowpass(highpass(noise(0.11, 2), 400), 3200), 0.001, 42)
    thump = sweep(0.13, 210, 62, 38, 30, ((1.0, 1.0), (2.0, 0.25)))
    return soft_clip(mix((click, 0.6), (crack, 0.9), (thump, 1.0)), 1.8)


def hit_magic():
    # Bright energy pop: crystalline zap gliding down + airy noise.
    zap = sweep(0.15, 1900, 520, 30, 26, ((1.0, 1.0), (1.5, 0.35), (2.0, 0.2)))
    air = envelope(highpass(noise(0.12, 3), 2200), 0.001, 36)
    body = sweep(0.11, 260, 110, 30, 34)
    return soft_clip(mix((zap, 0.65), (air, 0.45), (body, 0.7)), 1.5)


def crit():
    # Sharp metallic ring on top of a doubled punch.
    punch = hit_physical()
    ring = sweep(0.32, 2650, 2380, 8, 12, ((1.0, 1.0), (2.76, 0.42), (5.4, 0.18)))
    snap = envelope(highpass(noise(0.03, 4), 4000), 0.0003, 160)
    return soft_clip(mix((punch, 1.0), (ring, 0.42), (snap, 0.7)), 2.0)


def kill():
    # Crunchy pop: short broken-shell burst with a falling body.
    crunch_raw = noise(0.2, 5)
    crunch = [
        value * (1.0 if (n // 180) % 3 else 0.35)
        for n, value in enumerate(one_pole_lowpass(crunch_raw, 2600))
    ]
    crunch = envelope(crunch, 0.001, 22)
    body = sweep(0.2, 340, 70, 22, 18, ((1.0, 1.0), (2.0, 0.3)))
    click = envelope(highpass(noise(0.01, 6), 3000), 0.0003, 420)
    return soft_clip(mix((crunch, 0.85), (body, 1.0), (click, 0.5)), 2.2)


def kill_heavy():
    # Elite and boss kills: sub boom, rumble tail and a debris crack.
    boom = sweep(0.75, 120, 34, 9, 5.2, ((1.0, 1.0), (2.0, 0.22)))
    rumble = envelope(one_pole_lowpass(noise(0.75, 7), 420), 0.004, 5.5)
    debris = envelope(one_pole_lowpass(highpass(noise(0.35, 8), 900), 5000), 0.001, 13)
    click = envelope(highpass(noise(0.015, 9), 2400), 0.0004, 300)
    return soft_clip(mix((boom, 1.0), (rumble, 0.9), (debris, 0.55), (click, 0.5)), 2.4)


def hero_hurt():
    # Dull hurt thud with a short dissonant buzz so it reads as "you got hit".
    thud = sweep(0.22, 150, 55, 20, 16)
    buzz = sweep(0.18, 233, 196, 12, 20, ((1.0, 1.0), (1.06, 0.8), (3.0, 0.25)))
    grit = envelope(one_pole_lowpass(noise(0.12, 10), 1600), 0.001, 30)
    return soft_clip(mix((thud, 1.0), (buzz, 0.45), (grit, 0.5)), 1.9)


def gate_hit():
    # Heavy blow on timber and stone: low knock, woody body and grit spray.
    knock = sweep(0.32, 95, 48, 16, 11, ((1.0, 1.0), (2.3, 0.35)))
    timber = sweep(0.22, 340, 260, 10, 18, ((1.0, 1.0), (1.47, 0.6), (2.9, 0.25)))
    grit = envelope(one_pole_lowpass(highpass(noise(0.3, 11), 600), 3800), 0.001, 16)
    click = envelope(highpass(noise(0.012, 12), 2000), 0.0004, 320)
    return soft_clip(mix((knock, 1.0), (timber, 0.45), (grit, 0.5), (click, 0.4)), 2.0)


if __name__ == "__main__":
    AUDIO_DIR.mkdir(parents=True, exist_ok=True)
    write("hit_physical.wav", hit_physical())
    write("hit_magic.wav", hit_magic())
    write("crit.wav", crit())
    write("kill.wav", kill())
    write("kill_heavy.wav", kill_heavy())
    write("hero_hurt.wav", hero_hurt())
    write("gate_hit.wav", gate_hit())
