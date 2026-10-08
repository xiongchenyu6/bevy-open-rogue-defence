# Finite zone pulses and banner repairs

The controlled fixture runs the actual zone and banner systems through a persistent Update schedule, so SelectedHero change ticks do not incorrectly trigger cleanup on every test invocation. A fresh run_system_once fixture did exactly that; its zero banner repairs are not accepted as gameplay evidence. Each effect has 1.4 seconds of remaining life, its next pulse/repair due in 0.15 seconds, and 3.2 seconds of total simulation. Targets remain alive at the zone center; collision and skill casting are excluded. Output is real SkillStrike events with nominal damage 7 per pulse and actual gate repair from 50/100 HP. The banner is a real pooled shell, and retirement must remove its behavior, hide the shell and restore armor/attack-speed bonuses.

| Step | Before storm/glacier/vortex pulses | Before gate repair | After pulses | After gate repair |
| --- | --- | --- | --- | --- |
| 1/60 s | 3 / 2 / 3 | 2.9000015 | 3 / 2 / 3 | 2.9000015 |
| 32/60 s | 2 / 2 / 2 | 1.4500008 | 3 / 2 / 3 | 2.9000015 |
| 32/15 s | 0 / 0 / 0 | 0 | 3 / 2 / 3 | 2.9000015 |
| Single 3.2 s step | 0 / 0 / 0 | 0 | 3 / 2 / 3 | 2.9000015 |

Zones and banners now consume only their living portion of a step, then retire. Already expired zones cannot damage or pull. Vortex pull uses living time and clamps to the distance to its center, rather than the old per-frame 0.1-second movement cap. Pause/final-fragment tests verify one valid last pulse/repair and only 0.038 units of pull in the last 0.01 seconds. A pre-existing radius test caught an initial regression that removed a still-live zone on a zero-time step; that regression was corrected and the unchanged radius test now passes.

Final production source passed 607 library tests, 0 failed, 1 existing ignored; strict all-target Clippy and both binary builds passed. Five native cases completed 2000 frames: skills 12/14/15/21 at 1280×720 and skill15 at 667×375. Ten screenshots were inspected. Actual charged casts, learned-skill visuals, damage, paused projection transforms and finished projection cleanup checks passed. [Evidence](verification/zone-expiry-2026-10-09/verification.json). Its first attempt failed because the old identity harness paused basic attacks, disabling the actual charge source for damage spells. The updated harness keeps normal basic attacks and asserts actual charged zone cast telemetry, instead of manufacturing charge or bypassing cooldowns.

This is finite zone/banner lifetime output evidence, not whole-game speed equivalence, status-control timing equivalence or hardware performance. Other skill cooldowns, burn lifetime ticks and attack backlogs remain to be checked. Visual inspection still shows a thick flat vortex spiral and coarse glacier-control wire cages; this is not final artwork. The e109893 packaged first-chapter clear predates this source change; the existing Linux package does not yet contain this correction.
