#!/usr/bin/env bash
set -euo pipefail
out="$PWD/target/verification/window-perf-desktop-2026-11-05"
export WINIT_UNIX_BACKEND=wayland WAYLAND_DISPLAY=wayland-1 XDG_RUNTIME_DIR=/run/user/1000
export XDG_CONFIG_HOME="$out/state/config" XDG_CACHE_HOME="$out/state/cache"
export BEVY_ASSET_ROOT="$PWD" VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
export ROGUE_CAMPAIGN_SAVE="$out/state/campaign.save" ROGUE_AUDIO_SAVE="$out/state/audio.save" ROGUE_TUTORIAL_SAVE="$out/state/tutorial.save" ROGUE_DETAIL_SAVE="$out/state/detail.save"
export ROGUE_COMMANDER_SEED=0xA11CE ROGUE_COMMANDER_LANG=en ROGUE_PERF_PRESENT_MODE=fifo
export ROGUE_PERF_BUILD_PROFILE='cargo --release; standard repository profile'
export ROGUE_PERF_PURPOSE='actual Wayland desktop window control; native present cadence, not displayed-frame timestamps'
export ROGUE_PERF_CONCURRENT_WORKLOAD='No project build or capture workload; existing compositor/desktop/browser services remain'
ps -eo pid,etime,%cpu,%mem,comm --sort=-%cpu > "$out/processes-before.txt"
if pgrep -x 'rustc|cargo' > "$out/concurrent-builds.txt"; then exit 1; fi
nvidia-smi --query-gpu=name,driver_version,utilization.gpu,temperature.gpu,power.draw --format=csv > "$out/gpu-before.csv"
timeout -k 5s 130s nix develop --command target/release/perf_probe 32 10 30 "$out/desktop-32x" > "$out/desktop-32x.log" 2>&1
if rg -q 'panicked|PERF_INVALID|PERF_TIMEOUT|Encountered a panic' "$out/desktop-32x.log"; then exit 1; fi
rg -q PERF_PASS "$out/desktop-32x.log"
python tools/desktop/verify_perf_probe.py "$out/desktop-32x" --minimum-seconds 30 --surface 1920x1080 > "$out/desktop-32x-verified.json"
nvidia-smi --query-gpu=name,driver_version,utilization.gpu,temperature.gpu,power.draw --format=csv > "$out/gpu-after.csv"
