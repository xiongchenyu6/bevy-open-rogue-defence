set -euo pipefail
while true; do
 state=$(systemctl --user show rogue-archive-hud-checks-oct9.service -p ActiveState --value)
 case "$state" in inactive|failed) break;; *) sleep 5;; esac
done
[ "$(systemctl --user show rogue-archive-hud-checks-oct9.service -p ExecMainStatus --value)" = 0 ]
rg -q 'Finished `dev` profile' target/verification/archive-hud-build-oct9.log
dest="$PWD/target/verification/archive-hud-oct9"
mkdir -p "$dest/config" "$dest/runtime"
chmod 700 "$dest/runtime"
printf '1 0 0 1\n' > "$dest/audio.save"
for lang in en zh-CN; do
 for size in 1280x720 667x375; do
  case_dir="$dest/$lang-$size"; mkdir -p "$case_dir"
  env -u ROGUE_TUTORIAL_SAVE VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json WGPU_BACKEND=vulkan XDG_CONFIG_HOME="$dest/config" XDG_RUNTIME_DIR="$dest/runtime" ROGUE_AUDIO_SAVE="$dest/audio.save" ROGUE_CAMPAIGN_SAVE="$case_dir/campaign.save" ROGUE_COMMANDER_LANG="$lang" ROGUE_CAPTURE_SAVE_FRAMES=89,149,229 nix develop --command target/debug/capture frames "$case_dir/frames" 260 archive-hud 0xBEEF "$size" > "$case_dir/capture.log" 2>&1
  printf 'ARCHIVE_HUD_NATIVE_PASS %s %s\n' "$lang" "$size"
 done
done
