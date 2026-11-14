#!/usr/bin/env bash
# usage: cap.sh BINARY OUTDIR NAME MODE FRAMES SCENARIO SIZE  (env passes through)
set -u
B=$1; P=$2; N=$3; mkdir -p "$P/saves"
export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json
export ROGUE_CAMPAIGN_SAVE=$PWD/$P/saves/$N-c.save ROGUE_TUTORIAL_SAVE=$PWD/$P/saves/$N-t.save ROGUE_AUDIO_SAVE=$PWD/$P/saves/$N-a.save
out=$P/$N; [ "$4" = still ] && out=$P/$N.png
timeout 600 nix develop --command "$B" "$4" "$out" "$5" "$6" 0xA11CE "$7" > "$P/$N.log" 2>&1
echo "$N exit $?"
