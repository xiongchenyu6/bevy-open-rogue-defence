#!/usr/bin/env bash
V=target/verification/combat-visual-2026-11-13; B=$V/capture-head; P=$V/after
export ROGUE_CAPTURE_ATTACK_TRACE=1 ROGUE_CAPTURE_RELEASE_TRACE=1 RUST_LOG=info,rogue_commander::projectile_hit=trace
for k in ember storm sword; do ROGUE_CAPTURE_SAVE_FRAMES=$(seq -s, 0 60) $V/cap.sh $B $P attack-$k frames 210 attack-$k 1280x720; done
for k in archer mage; do ROGUE_CAPTURE_SAVE_FRAMES=$(seq -s, 9 40) $V/cap.sh $B $P portrait-$k frames 250 weapon-growth-$k-starter 720x1280; done
ROGUE_CAPTURE_SAVE_FRAMES=9,10,11,12,14,18,46,60,90,91,92,120,121,122,150,152,165 $V/cap.sh $B $P hit-feedback frames 180 hit-feedback 1280x720
unset ROGUE_CAPTURE_ATTACK_TRACE ROGUE_CAPTURE_RELEASE_TRACE RUST_LOG
for s in standard final showcase; do for z in 1280x720 720x1280; do $V/cap.sh $B $P $s-$z still 240 $s $z; done; done
$V/cap.sh $B $P standard-390x844 still 240 standard 390x844
echo ALL DONE
