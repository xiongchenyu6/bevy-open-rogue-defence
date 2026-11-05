# Readable native aim at equal fronts

This is an intentional target-selection design refinement. The previous rule legally chose the lowest entity ID when priority/front score tied. The [actual complete replay audit](../designated-fix-replay-2026-11-06/README.md) showed that this could aim at different visible lanes under different allocations. It was not an illegal contact or a damage-formula defect.

Existing tier and front-X priority stay unchanged. Only equal-score selection now prefers the nearest enemy to the hero, then stable visible coordinates Z→X→Y, with entity ID last for otherwise coincident actors. No damage, attack speed, range, priority tier, input flow or targeting framework was changed. [Minimal policy patch](minimal-design.patch) and source hashes preserve the exact before/after boundary.

The regression calls real production `auto_attack` in paired worlds with the three recorded front positions and reversed actor allocation. It checks **actual `BasicAttackAim` and the actual spawned native projectile XZ direction**, not a duplicated comparator. Before the refinement, the selected lane changes:

| Allocation | Selected Z | Actual projectile normalized XZ |
| --- | --- | --- |
| Forward | 1.6940259 | (0.9430801, 0.3325658) |
| Reverse | 0.3940259 | (0.9966531, 0.0817481) |

A symmetric ±Z design case also fails the new stable-coordinate expectation under the old rule. A more-forward but farther target control passes in both allocations, preserving the original front priority. The focused old-policy run records **8 passed / 2 failed**, with the two new policy expectations failing. Raw logs are unchanged. The wrapper itself rejected an interleaved `--nocapture` literal failure line; [independent validation](before-validation.json) checks the actual failure section, exit 101 and passing priority control. This wrapper parsing issue is not a game failure and no test run/log was fabricated or rewritten.

After the refinement, the full optimized library suite records **681 passed, 0 failed, 1 ignored**. Strict all-target release Clippy and the optimized simulator build pass. Both actual aim/direction cases and the front-priority control pass. Logs, before hashes and matching pre/post-build source manifests accompany the [frozen binary receipt](binary.json).

A new normal duplicate pair with all actor/contact data from frame 1 through terminal is pending at this archive boundary. Passing these real aim tests proves the scoped new policy, not complete game determinism. No automatic 24-case matrix or numerical balance tuning was performed.
