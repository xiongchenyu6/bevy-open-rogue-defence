# Designated strike target lock

A non-area typed skill strike could damage a neighboring elite instead of its designated target. The real regression places a normal target at zero and an elite at X=0.7: the neighbor is inside its enlarged collision radius but outside a normal radius. Reversing allocation order changes relative entity bits while preserving geometry and actor attributes.

The fixture runs production typed-strike scheduling → actual pooled projectile creation → authoritative collision. Before the fix, normal/elite health is **[1000,900]** in one allocation order and **[900,1000]** in the other, with the same RNG state 11083784628768264341. [Before output](before-tests.log) records 0 passed / 1 failed (test exit 101), solely at the designated-target health assertion. The previous full replay's elite flag was not recorded; this fixture explicitly creates a real elite and independently proves the interception defect.

`ProjectileTarget(Entity)` now restricts designated carriers to that exact entity. Non-area `SkillStrike` creation attaches it. Dead or missing targets cannot fall through to nearby enemies. Area strikes and ordinary unrestricted projectiles retain their spatial behavior. Existing market/melee target restriction is consolidated into the same strict lock; obsolete candidate blacklist construction/arguments were removed, including the single basic-attack caller argument. Hit memory remains for actual repeat-hit/ricochet tracking. There is no compatibility layer, comparator change, new damage formula or public Game API.

The lock is removed on projectile retirement/rent for both visible shells and logic-only fallback reuse, and shared metadata reset includes it. Regression coverage uses real systems for:

- Both designated/neighbor allocation orders.
- Area hits on both overlapping bodies and ordinary unrestricted interception.
- Missing/dead target rejection with unchanged RNG, followed by normal expiry/reset.
- A retired designated carrier reused as an unrestricted shot that actually damages a new target.
- Existing visible/logic-only pool reset fixtures extended with lock metadata.

The complete optimized library suite records **678 passed, 0 failed, 1 ignored**; strict all-target release Clippy and the optimized simulator build pass. [Logs](after-full-tests.log), [Clippy](after-clippy.log), source manifests and [binary receipt](binary.json) retain verification. The original negative fixture was moved into its focused child test file after reproduction; the failure log remains unchanged.

Focused reproduction:

```sh
cargo test --release --lib nonarea_typed_strike_cannot_be_intercepted_by_overlapping_elite -j2 -- --nocapture
```

A new complete duplicate pair using the same frozen executable, actor trace from frame 1 through terminal and actual contact trace is **pending at this archive boundary**. This proves the isolated designated-target defect and scoped fix; it does not claim full-game determinism or identify every upstream allocation difference. No 24-case matrix or numerical tuning followed.
