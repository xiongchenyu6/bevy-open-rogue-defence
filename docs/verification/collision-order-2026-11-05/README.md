# Stable stationary projectile collisions

Stationary single-hit projectiles previously accepted enemy query order. Adding a harmless cosmetic component can move enemies between ECS archetypes and reorder that query, changing which overlapping enemy takes damage. Moving projectiles already sorted candidates by segment entry progress, then entity bits.

All projectile collision candidates now use that existing comparator. No damage values, hit geometry, RNG rules, or attack timing were changed. `react_to_hits` HashMap behavior is unchanged.

The regression runs the production pooled projectile and `projectile_hits` system against two durable enemies with identical positions. The worlds have identical actor IDs, health, projectile data and RNG; only the insertion order of a test-only cosmetic marker differs. This models archetype migration without exposing private `HitPunch` construction fields.

Before the fix, the stationary projectile produced health `[75,100]` versus `[100,75]`; the moving control produced `[100,75]` in both worlds. Both consumed the same RNG state. The focused run recorded **1 passed, 1 failed** in [before-tests.log](before-tests.log).

After the fix, the complete optimized library suite, including both regressions, recorded **672 passed, 0 failed, 1 ignored** in [after-full-tests.log](after-full-tests.log). Strict all-target release Clippy passed in [after-clippy.log](after-clippy.log). Source and evidence hashes are in [verification.json](verification.json). The full verification run also contains independently scoped inventory layout work; its completion is not evidence about collision behavior beyond these tests.

Focused command:

```sh
cargo test --release --lib collision_ignores_cosmetic_archetype_insertion_order -j2 -- --nocapture
```

This proves the isolated stationary collision defect and its regression coverage. It does not prove that cosmetic migration caused the separately observed full balance replay divergence, establish complete simulation determinism, or verify native visual appearance. Current-source duplicate replay and matrix gating remain separate work.
