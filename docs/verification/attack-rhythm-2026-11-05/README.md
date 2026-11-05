# Sword attack contact calibration

The sword could deal damage while its authored model was still raising the
weapon. All melee weapons previously used the same 42% action-window contact.
The sword now uses 60%, after the shared clip's rapid downswing. Attack interval,
damage, range, cleave rules and the other weapon timings retain their values.
Moving enemies are still checked at actual contact; the later first contact is
an intentional behavior change, not a claim that campaign balance is identical.

## Evidence

The baseline uses gameplay from `3fec882` with only the diagnostic capture trace
added. Native runs use production attacks and real target HP, not injected hit
events. They suppress learned skills and ultimates and use stationary high-HP
fixture enemies, one fixed camera and a 30Hz manual simulation step. These are
not earned campaign runs. RTX 3070 Vulkan rendered each 210-frame case.

`clip-audit.py` reads the actual GLB channels and transform hierarchy. Stone
Warden and Solar Vanguard have identical one-handed-chop animation channels.
At source phase .42 the sword hand slot is still high (model-space y=.923),
whereas at .60 it has descended to .588. This is bone evidence, not a weapon-tip
collision test or an authored contact event. The sampler explicitly uses NLERP
between quaternion keys; native animation playback is the stronger timing record.

| Actual sword observation | Before | After |
| --- | --- | --- |
| First damage frame | 16 | 19 |
| Source clip seek at damage | .452978s | .724764s |
| Observed source phase | .424667 | .679467 |
| Hits in first 80 observed frames | 6 | 6 |
| Inter-hit frame intervals | 12,12,12,11,12 | 12,12,12,11,12 |

The nominal .60 contact becomes an observed .679 phase in this 30Hz fixture due
to simulation-step quantization. Every paired hit deals the same damage and
leaves the same target HP; each shifts three frames. Solar's actual positive
damage observations also occur in the later downswing phase. The archer and mage
control cases preserve all 80 HP/damage/playback observations exactly.

The primary agent viewed the baseline early-contact frame, the changed windup
and contact frames, Solar's lowered sword and both ranged control scenes.
Screenshot requests are asynchronous; use the synchronized `ATTACK_TIMING`
logs for exact frame/clip comparisons rather than inferring a timestamp from a
PNG filename. `compare.py` verifies the paired observations, and `comparison.json`
records its result.

## Reproduction and limits

From the repository root:

```sh
python docs/verification/attack-rhythm-2026-11-05/clip-audit.py
python docs/verification/attack-rhythm-2026-11-05/compare.py
ROGUE_CAPTURE_ATTACK_TRACE=1 ROGUE_CAPTURE_SAVE_FRAMES=20,25,30,32,34,36,38,40,45,50,55,60 \
  nix develop --command target/market-visual/debug/capture frames \
  target/verification/attack-rhythm-2026-11-05/after-sword \
  210 attack-sword 0xA11CE 1280x720
```

Use isolated campaign/tutorial/audio save environment paths and the appropriate
Vulkan ICD. Repeat `attack-ember`, `attack-storm` and `attack-solar`. The current
source produces the after case; archived baseline logs preserve the earlier
behavior. The diagnostic trace is opt-in for the capture binary and reads real
HP/playback without changing gameplay.

Build profile: dev and dependency opt-level 0, debug info 0, one Cargo job.
This verifies visual/gameplay timing, not release performance. Full library
tests: 658 passed, zero failed, one ignored. Strict Clippy passed. These records
do not establish an exact release point for every other weapon or certify all
combat feel complete.
