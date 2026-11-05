# Complete duplicate after target lock: numeric trajectories still differ

Two valid uncensored normal runs use the same frozen simulator (`65d4f11b6497dcf0bd0a26fdaf85a8c55afe006f8e9ca51b97b106cf81a23df1`) with the [designated-target lock](../designated-strike-2026-11-06/README.md). Seed A11CE, chapter index 5, no upgrades, active starter policy 2 and behind-gate position remain unchanged. Actor/contact tracing covers every update from frame 1 to normal terminal; there is no omitted initial interval.

| Run | Result | Terminal frame / actor rows | Battle seconds | Deaths | Kills | Wall seconds |
| --- | --- | --- | --- | --- | --- | --- |
| A | Win | 97999 | 1540.926 | 1 | 2081 | 185.13 |
| B | Win | 97841 | 1538.358 | 1 | 2162 | 285.68 |

Both wins are actual outcomes, not proof of equivalent trajectories or population win rate. Strict identities differ at frame 24. Every recorded nonidentity scalar and actor numeric multiset matches through frame **7923**; at **7924** the first numeric difference is a newly spawned native projectile's direction. Hero Storm Savant is level 3, native attack count rises 99→100 in both, casts remain 43 and RNG remains equal. No resolver contact occurs at this exact elapsed value (129.899765 seconds).

Three front enemies share the exact X bits representing **-3.3961327076**, health 192.5 and numeric positions in both runs. Their Z coordinates are 0.39402592, 1.04402590 and 1.69402587. From hero X=-8.2, A's actual velocity points to the closest/lower-Z enemy, while B's points to the farther/high-Z enemy. In each run that enemy has the lowest entity bits among the front tie. The [read-only audit](audit-native-aim.py) maps actual projectile direction to recorded positions and retains exact X bits in [values](native-aim-audit.json).

This matches the then-current explicit ordinary-attack rule: preserve tier/front-X priority and resolve equal score by entity ID. Choosing either tied front target was **legitimate prior game behavior**, not an illegal hit or the designated-strike defect returning. It explains why that rule permits visible aim to depend on allocation identity. A proposed nearest-then-visible-coordinate refinement is an intentional game-design improvement and requires its own before/after tests; none is silently included in these frozen runs.

Every raw stdout/contact stderr field is retained in deterministic gzip (mtime zero, empty filename), with compressed/decompressed hashes and direct row counts in [raw-receipts.json](raw-receipts.json). Correct counts: actor rows **97999 / 97841**, actual contacts **124551 / 155529**. The manager now labels `actor_trace_counts` and `contact_counts` separately. Original receipts, full comparison, predecessor context, manager and binary hash accompany the archive; redundant enormous all-contact derived files are not duplicated.

The scoped target-lock regression remains proven with 678 library tests and strict Clippy. Complete fixture numeric reproducibility remains unestablished. No 24-case matrix, numerical tuning, target-priority change or additional gameplay fix followed this pair in the archived executable.
