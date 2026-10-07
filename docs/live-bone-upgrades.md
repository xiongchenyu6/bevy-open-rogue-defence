# Live bone guard upgrades

Previously, permanent bone guards kept the damage calculated when summoned. Training changed their visible size, but an existing guard's attack damage did not follow its new rank or inherited hero attributes.

Existing guards now refresh damage from the production skill formula during active simulation. This preserves the entity, summon count, position, and attack cooldown. Repeated updates assign the current value rather than multiplying the previous value. Paused simulation does not update guards.

The Chinese description now lists attack, size, and simultaneous summon capacity. It no longer promises increased health: bone guards currently have no health component.

## Verification

The real ECS regression test starts with one existing guard and checks rank 1 damage of 24, rank 2 damage of 35, and damage of 39.75 after intelligence increases to 10. Ten subsequent updates preserve that value, the original entity and its 0.73-second attack timer. Pausing prevents a further rank change from applying.

- Library tests: 567 passed, 0 failed, 1 existing ignored.
- `cargo fmt --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.

Logs are archived in `verification/live-bone-upgrades-2026-10-07/`. This change has not yet been verified through a new native capture or a complete campaign balance run. Earlier summon formation captures do not include this damage change.
