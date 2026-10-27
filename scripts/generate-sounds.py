#!/usr/bin/env python3
"""Generate the built-in notification sounds in assets/sounds/.

The sounds are synthesized here (no third-party audio), so they ship under
the project's MIT license. Run from the repo root:

    python3 scripts/generate-sounds.py
"""
import math
import struct
import wave
from pathlib import Path

SAMPLE_RATE = 22_050
OUT_DIR = Path(__file__).resolve().parent.parent / "assets" / "sounds"


def tone(samples, seconds, freq):
    """Square-ish tone (clipped sine) whose frequency may change over time."""
    n = int(seconds * SAMPLE_RATE)
    fade = int(0.005 * SAMPLE_RATE)  # avoid clicks at the edges
    phase = 0.0
    for i in range(n):
        t = i / SAMPLE_RATE
        phase += math.tau * freq(t) / SAMPLE_RATE
        value = max(-1.0, min(1.0, math.sin(phase) * 3.0))
        envelope = min(1.0, min(i, n - i) / fade)
        samples.append(value * envelope * 0.6)


def silence(samples, seconds):
    samples.extend([0.0] * int(seconds * SAMPLE_RATE))


def bell(samples, seconds, base, partials):
    """Struck-glass tone: decaying sine partials of `base` (ratio, gain, decay)."""
    n = int(seconds * SAMPLE_RATE)
    attack = int(0.002 * SAMPLE_RATE)
    for i in range(n):
        t = i / SAMPLE_RATE
        value = sum(
            gain * math.exp(-t * decay) * math.sin(math.tau * base * ratio * t)
            for ratio, gain, decay in partials
        )
        samples.append(value * min(1.0, i / attack) * 0.85)


def chime():
    """Short, bright notification ding (the default on Linux and Windows)."""
    s = []
    bell(s, 1.2, 1_318.5, [(1.0, 0.7, 4.0), (2.0, 0.25, 6.0), (3.0, 0.12, 9.0), (4.2, 0.06, 12.0)])
    return s


def alarm():
    """Alarm clock: rapid bursts of a harsh high beep."""
    s = []
    for _ in range(4):
        for _ in range(4):
            tone(s, 0.07, lambda t: 2_400.0)
            silence(s, 0.05)
        silence(s, 0.2)
    return s


def siren():
    """Wailing siren sweeping up and down."""
    s = []
    for _ in range(3):
        tone(s, 0.6, lambda t: 600.0 + 900.0 * (t / 0.6))
        tone(s, 0.6, lambda t: 1_500.0 - 900.0 * (t / 0.6))
    return s


def klaxon():
    """Two-tone hi-lo emergency horn."""
    s = []
    for _ in range(4):
        tone(s, 0.3, lambda t: 960.0)
        tone(s, 0.3, lambda t: 770.0)
    return s


def write_wav(path, samples):
    with wave.open(str(path), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SAMPLE_RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1.0, min(1.0, x)) * 32767)) for x in samples))


if __name__ == "__main__":
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    for name, make in [("chime", chime), ("alarm", alarm), ("siren", siren), ("klaxon", klaxon)]:
        path = OUT_DIR / f"{name}.wav"
        write_wav(path, make())
        print(f"{path.relative_to(OUT_DIR.parent.parent)}  {path.stat().st_size // 1024} KB")
