#!/usr/bin/env bash
set -euo pipefail
out="$PWD/target/verification/window-perf-profile-child-2026-11-05"
mkdir -p "$out/state/runtime" "$out/state/config" "$out/state/cache"
chmod 700 "$out/state/runtime"
export DISPLAY=:93 WINIT_UNIX_BACKEND=x11
unset WAYLAND_DISPLAY
export XDG_RUNTIME_DIR="$out/state/runtime" XDG_CONFIG_HOME="$out/state/config" XDG_CACHE_HOME="$out/state/cache"
export BEVY_ASSET_ROOT="$PWD" VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
export ROGUE_CAMPAIGN_SAVE="$out/state/campaign.save" ROGUE_AUDIO_SAVE="$out/state/audio.save" ROGUE_TUTORIAL_SAVE="$out/state/tutorial.save" ROGUE_DETAIL_SAVE="$out/state/detail.save"
export ROGUE_COMMANDER_SEED=0xA11CE ROGUE_COMMANDER_LANG=en
export ROGUE_PERF_BUILD_PROFILE='cargo --release; standard repository profile'
export ROGUE_PERF_CONCURRENT_WORKLOAD='No Rust build observed at preflight; no capture readback; existing desktop/browser services remain; Xvfb/Openbox'
if test -S /tmp/.X11-unix/X93; then echo 'Display :93 already in use; refusing to change another display'; exit 1; fi
/run/current-system/sw/bin/nvidia-smi --query-gpu=name,driver_version,utilization.gpu,temperature.gpu,power.draw --format=csv > "$out/gpu-before.csv"
ps -eo pid,etime,%cpu,%mem,comm --sort=-%cpu > "$out/processes-before.txt"
if pgrep -x 'rustc|cargo' > "$out/concurrent-builds.txt"; then echo 'Concurrent Rust compilation; no timing collected'; exit 1; fi
/nix/store/s67rywcyd8xhvy4gq1l30scppidgksn2-xorg-server-21.1.24/bin/Xvfb :93 -screen 0 1600x900x24 -nolisten tcp -ac > "$out/xvfb.log" 2>&1 &
xvfb_pid=$!
wm_pid=''
cleanup() { if test -n "$wm_pid"; then kill "$wm_pid" 2>/dev/null || true; fi; kill "$xvfb_pid" 2>/dev/null || true; }
trap cleanup EXIT
for attempt in {1..50}; do if test -S /tmp/.X11-unix/X93; then break; fi; sleep .1; done
test -S /tmp/.X11-unix/X93
/nix/store/z62fkrxby0wqcisgxy9y78216lfxll5f-openbox-3.6.1/bin/openbox > "$out/openbox.log" 2>&1 &
wm_pid=$!
sleep 1
export ROGUE_PERF_PURPOSE='CPU-clock profiling enabled at observed sample begin; not baseline timing'
prefix="$out/profile-1x"
mkfifo "$out/control"
perf record -F 99 -e cpu-clock:u --user-callchains --call-graph dwarf,8192 -D -1 --control="fifo:$out/control" -o "$out/cpu.data" -- timeout -k 5s 130s nix develop --command target/release/perf_probe 1 10 30 "$prefix" > "$prefix.log" 2> "$out/perf-and-runtime.log" &
recorder_pid=$!
for attempt in {1..600}; do
 if rg -q PERF_SAMPLE_BEGIN "$prefix.log"; then break; fi
 kill -0 "$recorder_pid"
 sleep .1
done
rg -q PERF_SAMPLE_BEGIN "$prefix.log"
printf 'enable\n' > "$out/control"
date --utc --iso-8601=ns > "$out/sample-enable-time.txt"
wait "$recorder_pid"
rg -q PERF_PASS "$prefix.log"
perf report -i "$out/cpu.data" --stdio --no-children --sort comm,dso,symbol --percent-limit 0.5 > "$out/cpu-report.txt"
