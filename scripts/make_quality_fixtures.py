#!/usr/bin/env python3
"""Build transcript-quality fixtures from a LibriSpeech-style corpus.

Each fixture directory gets mic.wav (48 kHz mono 16-bit), an optional
system.wav, and reference.json with exact segment times and transcripts.
Clip choice is deterministic (sorted speaker and clip ids) so fixtures are
reproducible across machines.
"""

import json
import subprocess
import sys
import tempfile
import wave
from array import array
from pathlib import Path

RATE = 48000
QUIET_GAIN = 0.06
BLEED_GAIN = 0.12
BLEED_DELAY_MS = 100


def load_speakers(corpus_dir):
    speakers = {}
    for trans in sorted(corpus_dir.rglob("*.trans.txt")):
        for line in trans.read_text().splitlines():
            clip_id, _, text = line.partition(" ")
            flac = trans.parent / f"{clip_id}.flac"
            if flac.is_file() and text.strip():
                speakers.setdefault(clip_id.split("-")[0], []).append(
                    (clip_id, flac, text.strip())
                )
    for clips in speakers.values():
        clips.sort()
    return dict(sorted(speakers.items()))


def decode_clip(flac, tmp_dir):
    out = tmp_dir / f"{flac.stem}.wav"
    afconvert = ["afconvert", "-f", "WAVE", "-d", f"LEI16@{RATE}", "-c", "1", str(flac), str(out)]
    ffmpeg = ["ffmpeg", "-y", "-loglevel", "error", "-i", str(flac), "-ar", str(RATE), "-ac", "1", str(out)]
    for command in (afconvert, ffmpeg):
        try:
            if subprocess.run(command, capture_output=True).returncode == 0:
                break
        except FileNotFoundError:
            continue
    else:
        sys.exit(f"Could not decode {flac}: need afconvert or ffmpeg")
    with wave.open(str(out), "rb") as reader:
        assert reader.getframerate() == RATE and reader.getnchannels() == 1
        return array("h", reader.readframes(reader.getnframes()))


class Track:
    """A mono sample buffer with clips placed at millisecond offsets."""

    def __init__(self):
        self.samples = array("h")
        self.segments = []

    def append_clip(self, samples, text, source, gap_ms):
        self.pad_to(self.duration_ms() + gap_ms)
        start_ms = self.duration_ms()
        self.samples.extend(samples)
        self.segments.append(
            {"source": source, "start_ms": start_ms, "end_ms": self.duration_ms(), "text": text}
        )

    def duration_ms(self):
        return len(self.samples) * 1000 // RATE

    def pad_to(self, ms):
        target = RATE * ms // 1000
        if target > len(self.samples):
            self.samples.extend([0] * (target - len(self.samples)))


def scaled(samples, gain):
    return array("h", (int(sample * gain) for sample in samples))


def mix_bleed(mic, system, gain, delay_ms):
    offset = RATE * delay_ms // 1000
    for index, sample in enumerate(system):
        target = index + offset
        if target >= len(mic):
            break
        mixed = mic[target] + int(sample * gain)
        mic[target] = max(-32768, min(32767, mixed))


def write_fixture(out_dir, name, mic, system=None):
    fixture_dir = out_dir / name
    fixture_dir.mkdir(parents=True, exist_ok=True)
    segments = list(mic.segments)
    if system is not None:
        end_ms = max(mic.duration_ms(), system.duration_ms())
        mic.pad_to(end_ms)
        system.pad_to(end_ms)
        write_wav(fixture_dir / "system.wav", system.samples)
        segments += system.segments
    write_wav(fixture_dir / "mic.wav", mic.samples)
    segments.sort(key=lambda segment: (segment["start_ms"], segment["source"]))
    (fixture_dir / "reference.json").write_text(json.dumps({"segments": segments}, indent=1) + "\n")
    print(f"  {name}: {mic.duration_ms() / 1000:.1f}s, {len(segments)} reference segments")


def write_wav(path, samples):
    with wave.open(str(path), "wb") as writer:
        writer.setnchannels(1)
        writer.setsampwidth(2)
        writer.setframerate(RATE)
        writer.writeframes(samples.tobytes())


def monologue(clips, gap_ms, source="mic", min_ms=0):
    track = Track()
    for samples, text in clips:
        track.append_clip(samples, text, source, gap_ms)
        if min_ms and track.duration_ms() >= min_ms:
            break
    return track


def two_party(a_clips, b_clips, gap_ms=1500):
    mic, system = Track(), Track()
    for (a_samples, a_text), (b_samples, b_text) in zip(a_clips, b_clips):
        mic.append_clip(a_samples, a_text, "mic", gap_ms)
        system.pad_to(mic.duration_ms())
        system.append_clip(b_samples, b_text, "system", gap_ms)
        mic.pad_to(system.duration_ms())
    return mic, system


def main():
    corpus_dir, out_dir = Path(sys.argv[1]), Path(sys.argv[2])
    speakers = load_speakers(corpus_dir)
    eligible = [clips for clips in speakers.values() if len(clips) >= 8]
    if len(eligible) < 2:
        sys.exit(f"Need two speakers with 8+ clips in {corpus_dir}")
    speaker_a, speaker_b = eligible[0], eligible[1]

    print("Decoding clips...")
    with tempfile.TemporaryDirectory() as tmp:
        tmp_dir = Path(tmp)
        a = [(decode_clip(flac, tmp_dir), text) for _, flac, text in speaker_a[:16]]
        b = [(decode_clip(flac, tmp_dir), text) for _, flac, text in speaker_b[:3]]

    print(f"Building fixtures in {out_dir}...")
    write_fixture(out_dir, "1-clean-single", monologue(a[:3], gap_ms=1000))

    quiet = monologue([(scaled(samples, QUIET_GAIN), text) for samples, text in a[:3]], gap_ms=1000)
    write_fixture(out_dir, "2-quiet-speaker", quiet)

    mic, system = two_party(a[:3], b)
    write_fixture(out_dir, "3-call-two-party", mic, system)

    mic, system = two_party(a[:3], b)
    mix_bleed(mic.samples, system.samples, BLEED_GAIN, BLEED_DELAY_MS)
    write_fixture(out_dir, "4-call-with-bleed", mic, system)

    write_fixture(out_dir, "5-long-monologue", monologue(a[3:], gap_ms=150, min_ms=40_000))
    print("Done.")


if __name__ == "__main__":
    main()
