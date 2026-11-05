# Stable zone pulse target assignment

A real production Vortex Column pulse previously assigned shared critical RNG draws to different enemies after a harmless cosmetic component changed enemy archetype traversal order. The isolated regression proves this mechanism; it does **not** establish why traversal order changed in every full replay or implicate the `HitPunch` HashMap specifically.

The production fix sorts the collected affected targets by entity bits **before** scheduling pulse strikes. Damage, pulse interval, radius, duration, control and Vortex pull calculations are unchanged. The shared zone path uses this stable order for its pulse scheduling. A tiny `cfg(test)`-only bridge lets the regression run the actual zone update → typed strikes → pooled projectiles → authoritative collision, without a runtime public API.

The worlds allocate identical actors with identical attributes/transforms/RNG. Their only difference is cosmetic marker insertion order A/B versus B/A. The durable enemy starts at 1000 health; the weak one at 50. One recorded noncritical draw and one critical draw produce:

| Old production order | Durable health | Weak health after defeat | Typed strikes | Pulse interval | RNG after |
| --- | --- | --- | --- | --- | --- |
| A/B | 900 | -125 | 2 | 0.5 | 16263334312487546840 |
| B/A | 825 | -50 | 2 | 0.5 | 16263334312487546840 |

Both settle one kill, rent two actual pooled projectiles and consume them, retain a zero cast counter (the fixture starts an existing zone), and use the same recorded four-draw RNG budget. The old run fails **only** the final per-entity health invariant, not compilation or fixture counters. [Actual before output](before-tests.log) records 0 passed / 1 failed, test exit 101. [Source hashes](before-source-hashes.json) and the [minimal production patch](minimal-production-fix.patch) preserve the before/after boundary.

After the sort, the complete optimized library suite records **674 passed, 0 failed, 1 ignored**, including the causal regression. Strict all-target release Clippy and the optimized balance simulator build pass. Logs and matching pre/post-build source hash manifests are archived here; the [binary receipt](binary.json) identifies the frozen executable.

Focused reproduction:

```sh
cargo test --release --lib vortex_pulse_damage_ignores_cosmetic_enemy_archetype_order -j2 -- --nocapture
```

The prior [full-prefix trace](../full-prefix-replay-2026-11-06/README.md) independently records the actual target/critical assignment swap. A new normal duplicate pair is **pending** at this archive boundary, using the same new frozen binary and actor/contact tracing from frame 1 through terminal. Passing the isolated regression does not claim complete game determinism. No 24-case matrix or numerical tuning has run, and no additional production ordering changes are included.
