set -euo pipefail
while true; do
 state=$(systemctl --user show "${ARCHIVE_HUD_BUILD_UNIT:?final build unit required}" -p ActiveState --value)
 case "$state" in inactive|failed) break;; *) sleep 5;; esac
done
[ "$(systemctl --user show "${ARCHIVE_HUD_BUILD_UNIT:?final build unit required}" -p ExecMainStatus --value)" = 0 ]
rg -q 'Finished `dev` profile' "${ARCHIVE_HUD_BUILD_LOG:?final build log required}"
dest="$PWD/target/verification/archive-hud-oct9-r3"
mkdir -p "$dest/config"
archive_runtime_dir=$(mktemp -d /tmp/rc-archive-XXXXXX)
trap 'rm -rf "$archive_runtime_dir"' EXIT
chmod 700 "$archive_runtime_dir"
printf '1 0 0 1\n' > "$dest/audio.save"
for lang in en zh-CN; do
 for size in 1280x720 667x375; do
  case_dir="$dest/$lang-$size"; mkdir -p "$case_dir"
  env -u ROGUE_TUTORIAL_SAVE VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json WGPU_BACKEND=vulkan XDG_CONFIG_HOME="$dest/config" XDG_RUNTIME_DIR="$archive_runtime_dir" ROGUE_AUDIO_SAVE="$dest/audio.save" ROGUE_CAMPAIGN_SAVE="$case_dir/campaign.save" ROGUE_COMMANDER_LANG="$lang" ROGUE_CAPTURE_SAVE_FRAMES=89,149,229 nix develop --command target/debug/capture frames "$case_dir/frames" 260 archive-hud 0xBEEF "$size" > "$case_dir/capture.log" 2>&1
  printf 'ARCHIVE_HUD_NATIVE_PASS %s %s\n' "$lang" "$size"
 done
done
