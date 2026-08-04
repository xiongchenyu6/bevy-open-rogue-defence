#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
audio_dir="$project_dir/assets/audio"
mkdir -p "$audio_dir"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=0.24*sin(2*PI*(760+520*t)*t)*exp(-22*t):s=48000:d=0.16" \
  -c:a pcm_s16le "$audio_dir/shot.wav"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "anoisesrc=color=pink:amplitude=0.35:d=0.22:s=48000" \
  -af "lowpass=f=680,highpass=f=70,afade=t=out:st=0.03:d=0.19,volume=0.65" \
  -c:a pcm_s16le "$audio_dir/impact.wav"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=0.18*(sin(2*PI*659.25*t)+sin(2*PI*830.61*t)+sin(2*PI*987.77*t))*exp(-5*t):s=48000:d=0.58" \
  -af "afade=t=in:d=0.015,afade=t=out:st=0.30:d=0.28" \
  -c:a pcm_s16le "$audio_dir/draft.wav"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=0.16*(sin(2*PI*(180+620*t)*t)+0.6*sin(2*PI*(360+980*t)*t))*exp(-2.2*t):s=48000:d=0.95" \
  -f lavfi -i "anoisesrc=color=blue:amplitude=0.12:d=0.95:s=48000" \
  -filter_complex "[1:a]highpass=f=1200,afade=t=in:d=0.08,afade=t=out:st=0.55:d=0.40[n];[0:a][n]amix=inputs=2:normalize=0,volume=0.75[out]" \
  -map "[out]" -c:a pcm_s16le "$audio_dir/ultimate.wav"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=0.22*(sin(2*PI*(82-18*t)*t)+0.45*sin(2*PI*123*t))*exp(-1.25*t):s=48000:d=1.45" \
  -f lavfi -i "anoisesrc=color=brown:amplitude=0.09:d=1.45:s=48000" \
  -filter_complex "[1:a]lowpass=f=360,afade=t=out:st=0.7:d=0.75[n];[0:a][n]amix=inputs=2:normalize=0,volume=0.8[out]" \
  -map "[out]" -c:a pcm_s16le "$audio_dir/boss_spawn.wav"

ffmpeg -hide_banner -loglevel error -y \
  -f lavfi -i "aevalsrc=0.045*(sin(2*PI*55*t)+0.7*sin(2*PI*82.5*t)+0.32*sin(2*PI*110*t))+0.012*sin(2*PI*(220+7*sin(2*PI*t/12))*t):s=48000:d=24" \
  -af "afade=t=in:d=2,afade=t=out:st=22:d=2,lowpass=f=1800" \
  -c:a libvorbis -q:a 3 "$audio_dir/arena_loop.ogg"

echo "Generated original audio in $audio_dir"
