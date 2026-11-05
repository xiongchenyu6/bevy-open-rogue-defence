# After isolated zone fix: full duplicate gate still fails

Two normal complete replays used the same newly built frozen executable (`2c543f8f9b6800623e44e972ad0910c1f98519f7eb9961d0db633b84a14ff76d`) containing the [isolated zone-order fix](../vortex-order-2026-11-06/README.md). The parameters remain seed A11CE, chapter index 5, no upgrades, active starter policy 2 and behind-gate position. Actor tracing runs from frame 1 through each terminal update, with actual resolver contacts enabled. No production change followed the pair.

| Run | Valid uncensored result | Final frame / actor rows | Battle seconds | Deaths | Kills | Wall seconds |
| --- | --- | --- | --- | --- | --- | --- |
| A | Win | 97801 | 1538.420 | 0 | 2100 | 160.72 |
| B | Loss | 19308 | 316.482 | 3 | 126 | 27.01 |

Strict actor identities differ from frame 24. Every recorded nonidentity scalar and exact actor numeric multiset matches through frame **12715**; the first recorded numeric difference is enemy health at **12716**, battle elapsed 208.8138275 seconds. Removing IDs/order for this projection does not establish correspondence or equality of hidden state.

Each run has one actual resolver contact at that elapsed value: Storm Spell skill index 0, base damage 300.158203, final damage 345.181915, the same critical roll 0.44210544 and multiplier 1.0. The RNG stream, attack count 175 and cast count 78 match. The contacts hit different actual targets:

| Run | Target entity bits | Previous HP / max HP | Previous XYZ | After HP |
| --- | --- | --- | --- | --- |
| A | 4294963288 | 1160.3201 / 1160.3201 | (-2.882524, 0, 1.101806) | 815.1382 |
| B | 17179866065 | 1198.0916 / 2436.6721 | (-3.300000, 0, 1.770425) | 852.9097 |

Thus this new first difference is target selection/distribution, not the previously isolated zone critical reassignment. Exact predecessor/after rows and original contact lines are retained. Distance-only ordering in learned-skill primary/chain selection is a source-audit candidate, not a demonstrated tie or proven cause of this pair. No additional fix is justified by this archive alone.

Both stdout and stderr are losslessly gzip-compressed with empty filename and mtime zero; [raw-receipts.json](raw-receipts.json) gives independently counted rows and compressed/decompressed hashes. Correct actor counts are **97801 / 19308**, contact counts **98664 / 710**. The unmodified raw sequence's final `counts` field again contains contact counts due to a summary-label bug; it does not describe missing actor rows. No enormous redundant all-contact derived JSONL was archived.

The [isolated regression](../vortex-order-2026-11-06/README.md) still proves the scoped zone fix, with 674 library tests and strict Clippy passing. This pair does not establish complete fixture determinism. The 24-case numerical matrix remains withheld, and no numerical tuning or upstream HashMap change was performed.

## Read-only chain/collision audit

The cast telemetry is copied from runtime **before** learned skills cast during the update. Therefore the increment observed at 12711 corresponds to cast frame 12710. Using actual actor/contact data, the first contacted numeric primary is the same in both runs. From that primary at cast time, the next unvisited normal body is uniquely nearest: distance² **0.92233336**, versus **11.07991123** for the next candidate. There is no observed distance tie to justify a comparator patch.

A separate collision candidate exists: `resolve_skill_strikes` places a non-area stationary projectile at the intended target + Y 0.18, radius 0.05, one hit, without attaching target restriction. At resolution, the previous primary is distance² **0.65373814** from that strike point. This exceeds the normal hit radius² **0.2809**, but is within an elite hit radius² **1.0**. Its elite flag is **not recorded** in the actor trace, so this eligibility is conditional, not a directly logged flag claim.

If both overlap, existing entity ordering chooses the intended normal in A (4294963288 < 21474833370) and the neighboring primary in B (17179866065 < 30064767331), exactly matching observed contacts. [Reproducible read-only calculation](audit-chain.py) and [values](chain-distance-audit.json) retain actual cast/previous rows and the conditional geometry. A real designated-strike/nearby-elite regression is the next appropriate test; no production comparator or collision change is included in this archive.
