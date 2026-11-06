# Read-only attack-phase follow-up

Recommended next narrow gate: **starter ranged release vs authored anticipation**, first Ember archer, with Storm mage and accepted Sword as controls. This is a reachable unverified timing boundary, not an established defect or authorization to delay attacks.

## Actual source evidence

- `src/game/combat.rs:2094–2111` creates melee PendingSwing with accepted `swing_windup`; `basic_attacks.rs:596` retains Sword .60, Daggers .50, other melee .42 of clamped actual attack interval. Preserve those accepted calibrations.
- `combat.rs:2114–2159` instead rents ranged projectiles immediately during the resolved volley, at hero position +Y .25, with actual velocity/damage. There is no ranged authored-release delay here.
- `model_animation.rs:278–283` starts hero Attack from the SAME `AttackVolleyRuntime.resolved_this_frame > 0` cue. `311–340` fits the full authored clip into actual AttackClock duration (clamped .03..ATTACK_SECONDS). Therefore ranged projectile creation and animation start share the volley, whereas authored launch/hand pose has not been independently calibrated. This is not proof that the launch pose is wrong: authored animation may begin near release, and renderer/animation step quantization must be observed.
- `assets/models/cast-manifest.json` maps Storm to Mage/Spellcast_Shoot; inspect exact Ember mapped authored Attack before selecting a launch pose. Do not infer release timing from a target's distant damage frame: flight time separates release and contact.

## Existing native evidence and its limit

- `docs/verification/attack-rhythm-2026-11-05/README.md` explicitly states ranged controls retain 80 actual HP/damage/playback observations, and does NOT establish every other weapon's release point. Actual native logs/frames: `target/verification/attack-rhythm-2026-11-05/{after-ember,after-storm,after-sword}` plus corresponding `.log`.
- `docs/verification/starter-arrow-2026-11-05/README.md`: starter plain shaft/ironhead/no generic flight circle; actual one-target one-arrow and tier boundaries verified. This proves restrained shape/count, not bone-to-projectile launch timing.
- `docs/verification/dagger-rhythm-2026-11-06/` preserves accepted dual-blade calibration. No proposal to change it or Sword.

## Exact next bounded validation (NOT RUN)

After root explicitly releases source/Cargo/native lane, freeze current source/assets and capture binary hashes; old dated native controls are provenance, not current-source passes.

Reuse existing scenarios, each210updates, seed0xA11CE,1280x720, private campaign/tutorial/audio paths and actual models/pools:

`nix develop --command target/market-visual/debug/capture frames target/verification/attack-phase-followup-2026-11-06/current-ember 210 attack-ember 0xA11CE 1280x720`

Repeat attack-storm and attack-sword in distinct directories. Set `ROGUE_CAPTURE_ATTACK_TRACE=1` (confirm current opt-in name in attacks.rs), `ROGUE_CAPTURE_SAVE_FRAMES=8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40`.

The existing `ATTACK_TIMING` gives real HP and AnimationPlayer seek/speed, not first emission. A minimal capture-only observation should additionally log the first newly active ordinary projectile's actual entity/kind, origin/current position/velocity and the same hero animation seek each frame (no production delay, no fake HitEvent or HP). Record model-ready baseline so missing clip loading cannot masquerade as phase0. Identify two successive releases and corresponding attack clip cycles. Compare rendered hand/bow/staff pose at release frame and adjacent frames against authored GLB trajectory, keeping asynchronous PNG capture vs synchronous trace frame alignment explicit.

Pass/decision criteria: if actual launch occurs before a clearly authored draw/cast anticipation, propose only that weapon's smallest delayed-release fixture candidate with unchanged interval/count/damage/speed; do not uniformly apply melee .60. If authored initial pose already matches launch, record that and make no timing change. Check a subsequent cycle's recovery reaches intended pose before next volley; a single first-frame image cannot establish truncated recovery. Sword control retains accepted phase/contact trace; starter arrow remains one target/one ordinary projectile. This is bounded controlled rendering evidence, not natural progression or global combat-feel acceptance.

No source/assets/tools edited, no builds or native jobs launched during this audit.

## Actual current-source native observations

Root's unit `rogue-attack-phase-followup-nov6`, invocation `03f042a750ef418692defa27956a2105`, completed all three210update cases with exit0. Actual commands/results are `current-{ember,storm,sword}-result.json`; raw logs and PNG directories retain their names. `analyze-current.py` reads these logs only; `measured-summary.json` preserves first-two-cycle phase rows and hashes.

| Case | Last phase observations | Newly observed carrier epochs | First two Attack starts / first ranged observations |
|---|---:|---:|---|
| Ember |210|24|frames11/21; seek .10453333/.107146665, speed3.1360002/3.2144|
| Storm |210|13|frames16/31; seek .0691358/.0691358, speed2.074074/2.074074|
| Sword |210|0|Attack starts12/24; seek .09059555, speed2.7178667|

Canonical authored GLB Attack sampler max time: Ember1.0666666666666667s (`2H_Ranged_Shoot` imported as Attack), Storm.9333333333333333s (`Spellcast_Shoot` imported as Attack). Both use animation graph node3 (graph has root node0 and Idle/Walk/Attack/Hit1..4). The observed first Ember attack is about9.80% source clip, second10.04%; Storm about7.41%. This is the Last-observed phase after one animation advance, **not** a measured authored launch instant.

First Ember projectile observation x−1.3853333, y.25, velocity+x18.44; Storm x−1.3586667, y.25, velocity+x19.24. From fixture hero x−2, each is already one30Hz flight step ahead (about.61467/.64133). Entity2804v0 is reused across Ember's first two observations with distinct added ticks25192/32539; Storm entity2783v0 likewise ticks28865/39882. This demonstrates the epoch diagnostic catches pool reuse. Sword's zero observed carriers is expected with same-update strict contact carriers retired before Last; it supplies no projectile-release evidence and is not a failure.

Root viewed Ember PNG9/10/12/15: current production responsive BattleCamera overrides the fixture's intended10m view and leaves hero about40pixels high. These pixels do not resolve hand/bow motion sufficiently. **No release timing defect or calibration acceptance follows from this run.** Source clip advancement/cycle records alone also do not establish visually truncated recovery.

Next smallest diagnostic: under ONLY `ROGUE_CAPTURE_RELEASE_TRACE`, remove `BattleCamera` from the controlled capture Camera3d entities (existing `hit_feedback/capture.rs:65–71` pattern), then preserve attacks.rs's existing camera center, transform and FixedVertical10m projection. No gameplay/attack/timer edits. This makes the intended close-up owner explicit rather than repeatedly fighting production camera. Rebuild once after root authorization, rerun the same3×210cases and adjacent release/recovery PNGs, verify actual hero pixel size and pose readability before any timing proposal. Keep current broad-view logs as valid technical telemetry, not substitute hand-release visual proof.
