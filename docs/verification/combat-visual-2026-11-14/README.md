# Combat feel follow-ups and visual pass — native evidence (2026-11-14)

Source basis: gameplay/presentation code is commit `89c64a7` (weapon release,
hit reactions, cavern lighting) plus `40cfb93` (portrait recruit prompt, contact
light strength). The captured binary and simulator were built from the tree at
`c01e4c1` with the `40cfb93` changes applied (uncommitted at build time; see
`matrix/source-join.json`), so they also contain the unrelated skill-effect
commits `f64002a..c01e4c1`. "Before" is `cd1095d`. Binary hashes: `binary.sha256`.

All captures use the existing offscreen `capture` harness on NVIDIA Vulkan
(`scripts/cap.sh`, `run_before.sh`, `run_after.sh`). Fixtures are controlled
native scenes, not a played session.

## What changed

| Item | Change |
| --- | --- |
| Hit flash | Ordinary contacts no longer swap body materials. A pooled, unshadowed point light (12 fixed entities, 70 ms real time) lights only the attacker-facing side of the struck body. Critical contacts keep the warm whole-body variant and get a stronger gold light. |
| Release point | `tools/measure_weapon_socket.py` evaluates the authored Attack clip. Ember keeps phase 0.25 (crossbow recoil onset); Storm moves 0.28 → 0.33 (staff at full extension). The rendered projectile starts at the measured muzzle; `Transform`, collision lane (y 0.25) and damage are unchanged. |
| Release flash | The ranged release cue used to fire at attack commit; for delayed shots it now fires on the release frame at the muzzle. |
| Portrait | Cavern rock is toned dark and cool, a vignette and a violet mouth glow were added, and the recruit prompt no longer sits on the HUD. The roof is still in frame by design of the fixed camera. |

## Measured results (`trace-analysis.json`, from `scripts/analyze_traces.py`)

| Check (first 80 updates unless noted) | Ember | Storm | Sword (control) |
| --- | --- | --- | --- |
| Damage rows before / after (210 updates) | 23 / 23 | 13 / 13 | 51 / 51 |
| Rows identical in damage, crit roll bits, HP bits, RNG states | 23 | 13 | 51 |
| Real contact frames with a contact light on the same frame | 7 of 7 | 4 of 4 | 6 of 6 |
| Contact lights without a contact | 0 | 0 | 0 |
| Hit sound entity spawned on the contact frame | 7 of 7 | 4 of 4 | 6 of 6 |
| Shot sound spawned on the release frame (210 updates) | 24 of 24 | 13 of 13 | n/a |
| Rendered-minus-logic offset vs measured socket, max error | 1e-7 | 7e-8 | n/a |
| Logic projectile height at release | 0.25 (all) | 0.25 (all) | n/a |

Storm's contact still moves one update later than before (release update 19 →
20), because its phase changed; the damage rows themselves are unchanged.

The live weapon node at the first Ember release is (-0.0845, 0.8188, 0.3649)
from the hero (forward, up, right); the offline evaluator gives
(-0.064…-0.091, 0.819, 0.365) for that clip time, so the measurement tool
matches the rig on screen. The muzzle itself is the weapon mesh's forward
bounds face centre, not an authored socket node (the models have none).

At 30 Hz one update is about 10 % of the attack clip, so releases land at or
just after the configured phase (Ember clip time 0.27–0.40 s of 1.067 s).

## Pixels actually reviewed

`frames/` holds the pairs that were opened and looked at:

- `*-ember-release-f13`, `*-storm-release-f19/f20`: the arrow now leaves the
  crossbow front and the bolt leaves the staff head instead of the body centre.
- `*-ember-contact-f20`, `after-portrait-archer-contact-f20`: a small warm
  highlight on the struck enemy's near side on the damage-number frame.
- `*-ordinary-contact-f90`, `*-critical-contact-f120`: ordinary keeps the body
  colour with a one-sided highlight; critical is gold over the whole body.
- `*-standard-1280x720`, `-720x1280`, `-390x844`, `after-final-*`: darker
  cavern, vignette, mouth glow; recruit prompt clear of the HUD in portrait
  (before: it covered the HP bars at 390x844 and sat under the speed buttons
  at 720x1280).

## Gates

- `cargo test --release --lib`: 802 passed, 0 failed, 1 ignored (pre-existing).
- `cargo clippy --release --all-targets -- -D warnings`: exit 0.
- `cargo check --target wasm32-unknown-unknown --lib`: exit 0.
- `cargo fmt -- --check`: failed in the chain on four files that were already
  unformatted at `cd1095d`; they were formatted in `503c864` and the check then
  reported 0 diffs.

- Re-run at `503c864` after the formatting commit: tests 802 passed / 0 failed /
  1 ignored, Clippy exit 0, fmt check exit 0 (`logs/final.out`, `logs/final-*.log.gz`).

Logs: `logs/chain-*.log.gz`, `logs/chain.out`.

## 24-case pressure matrix rerun (`matrix/`)

Same dimensions as `../post-life-pressure-matrix-2026-11-08`: chapters 0/5,
starters 0/1/2, seeds 1/BEEF, passive/active, behind-gate, zero training,
180000 frames. All 24 cases ended valid, uncensored, exit 0, no timeout;
manager time 1302.5 s; host profile hash unchanged.

| Group | Active policy | Recruitment-only policy |
| --- | --- | --- |
| Chapter 0 | 6 wins, 0 deaths, min alive health 0.931–0.996 | 6 losses, 276.4–692.2 s |
| Chapter 5 | 6 wins, deaths 1,0,0,0,0,0 | 6 losses, 68.7–196.2 s |

These are two-seed fixed-policy descriptions, not a balance acceptance. The
binary also contains the later skill-effect commits, so differences from the
11-08 sample (chapter 5 active was 5/6) cannot be attributed to the release
timing change alone.

## Not verified

- No same-seed full-run repeat pair was rerun; the 93642-frame exact replay
  match in `docs/balance-replay.md` still belongs to `064fe0a`.
- No audio was listened to. Only the spawn frame of each audio entity was checked.
- Sword/axe/dagger windup and recovery were not re-examined beyond the sword
  control run completing; elemental skill VFX were changed by other commits and
  are not covered here.
- Web and Android rendering of the new lights/vignette were not run (wasm
  compiles only). No package was rebuilt.
- Elite, Boss and long-session feel, and real hardware frame time.
- With a target closer than the muzzle distance, the drawn projectile can
  start beyond the target; collision is unaffected. Not captured.
