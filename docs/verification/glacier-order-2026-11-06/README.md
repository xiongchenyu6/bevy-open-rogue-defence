# Glacier pulse ordering: actual local red/green regression

The [previous full repeat](../post-gate-cinder-repeat-2026-11-06/README.md) retains
an actual Glacier14 critical-assignment difference at update 81007. This change
lets Glacier reuse the existing StormMatrix center-distance/Z/X/Y/entity ordering
when scheduling its real zone pulse strikes. Vortex remains on its old branch.
Damage, radius, five-second lifetime, 0.8-second interval and per-target freeze
counters are unchanged. Coincident geometry still has an entity tie-breaker.

## Actual tests

`red.log` records two passing controls and one failed desired behavior:

- Same physical targets and IDs, reverse harmless cosmetic archetype row order:
  passes under the original identity sorting.
- Same physical positions and stats, reversed enemy entity allocations: fails,
  HP [9886.838,14935.336] versus [9935.336,14886.838], with the same final RNG state
  16263334312487546840. Both actual worlds run before comparing their results.
- Three real pulses, both allocation orders: first two pulses do not block the
  targets' actions; the third freezes both. Each pulse resolves and retires its
  actual pooled projectiles. This is a freeze threshold test, not expiry testing.

The fixtures enable only rank-one Glacier and use a controlled primary target.
They prepare real attack cadence and resolved-volley charge with the initial
cooldown explicitly elapsed, then call actual `cast_rare_skills`. The resulting
zone is asserted to be Glacier, lifetime 5 and initial pulse 0.8. Actual
`update_spell_zones` -> `resolve_skill_strikes` -> strict target-lock pooled
carriers -> `projectile_hits` performs the damage. No synthetic zone or direct
post-hit HP write substitutes for that chain. One cast, two designated strikes,
one critical/one ordinary hit and four actual crit/proc RNG draws are checked.

After the correction all four printed pairing controls produce
HP [9935.336,14886.838] with the same RNG state. The full library run records
**740 passed, zero failed, one existing ignored test**. Selected Rust files pass
format checks; strict release all-target Clippy also exits zero.

## Scope

This establishes the controlled local scheduling regression, not every target
configuration, skill, shared-primary choice, full GamePlugin schedule, rendered
play, learning UI, radius boundary or freeze expiration. It does not prove the
older 75c win/loss flip's root cause or complete cross-process determinism.
The previous full-run negative remains valid historical evidence. A new frozen
full replay pair is still required before claiming even that fixture matches.

The sequential validation receipts and unchanged source/asset/vendor input
manifests are archived alongside logs. `provenance.json` hashes the actual built
simulator, which is not stored in Git. `verify.py` checks archive integrity and
actual red/green receipts; it does not rerun the game or prove whole-run equality.
