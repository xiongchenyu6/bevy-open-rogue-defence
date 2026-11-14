#!/usr/bin/env bash
# usage: run_before.sh BINARY OUTDIR
set -u
B=$1; P=$2; mkdir -p $P
export VK_ICD_FILENAMES=/run/opengl-driver/share/vulkan/icd.d/nvidia_icd.json ROGUE_CAMPAIGN_SAVE=$PWD/$P/c.save ROGUE_TUTORIAL_SAVE=$PWD/$P/t.save ROGUE_AUDIO_SAVE=$PWD/$P/a.save ROGUE_CAPTURE_ATTACK_TRACE=1 ROGUE_CAPTURE_RELEASE_TRACE=1 RUST_LOG=info,rogue_commander::projectile_hit=trace
run(){ name=$1; shift; rm -f $P/c.save $P/t.save $P/a.save; "$@" > $P/$name.log 2>&1; echo "$name exit $?"; }
for k in ember storm sword; do ROGUE_CAPTURE_SAVE_FRAMES=$(seq -s, 0 60) run attack-$k nix develop --command $B frames $P/attack-$k 210 attack-$k 0xA11CE 1280x720; done
for k in archer mage; do ROGUE_CAPTURE_SAVE_FRAMES=$(seq -s, 9 40) run portrait-$k nix develop --command $B frames $P/portrait-$k 250 weapon-growth-$k-starter 0xA11CE 720x1280; done
ROGUE_CAPTURE_SAVE_FRAMES=9,10,11,12,14,18,46,60,90,91,92,120,121,122,150,152,165 run hit-feedback nix develop --command $B frames $P/hit-feedback 180 hit-feedback 0xA11CE 1280x720
for s in standard final showcase; do for z in 1280x720 720x1280; do run $s-$z nix develop --command $B still $P/$s-$z.png 240 $s 0xA11CE $z; done; done
echo ALL DONE
