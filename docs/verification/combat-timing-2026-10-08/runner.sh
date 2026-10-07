#!/usr/bin/env bash
set -euo pipefail
while systemctl --user is-active --quiet rogue-combat-timing-checks-oct8.service; do sleep 5; done
rg -q 'Finished `dev` profile' target/verification/combat-timing-build-oct8.log
dest="$PWD/target/verification/combat-timing-oct8"
mkdir -p "$dest/config" "$dest/runtime"
chmod 700 "$dest/runtime"
printf '1 0 0 1\n' > "$dest/audio.save"
for scenario in skill-spectacle-combo summon-assault; do
mkdir -p "$dest/$scenario"
env -u ROGUE_CAMPAIGN_SAVE -u ROGUE_TUTORIAL_SAVE VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json WGPU_BACKEND=vulkan XDG_CONFIG_HOME="$dest/config" XDG_RUNTIME_DIR="$dest/runtime" ROGUE_AUDIO_SAVE="$dest/audio.save" ROGUE_COMMANDER_LANG=en ROGUE_CAPTURE_SAVE_FRAMES=299,599 ROGUE_CAPTURE_TIMINGS="$dest/$scenario/timings.csv" nix develop --command target/debug/capture frames "$dest/$scenario/frames" 680 "$scenario" 0xBEEF 1280x720 > "$dest/$scenario/capture.log" 2>&1
printf 'COMBAT_TIMING_CASE_PASS %s\n' "$scenario"
done
