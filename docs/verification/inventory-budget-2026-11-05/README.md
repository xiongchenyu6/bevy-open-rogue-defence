# Bounded equipment inspection budgets

The archived run stopped on a harness deadline, without a terminal game result. Wind Boots (201 → 41, cost 160) and Spirit Drum (287 → 87, cost 200) had their purchased identity verified. Serrated Disc (316 → 136, cost 180) was purchased, but its inventory inspection did not finish. Four visits proved resumed countdowns; the fifth did not. The earlier verifier stopped after finding the new item and did not freshly verify every prior identity.

The fifth visit captured the open inventory at 114.991 seconds, before its scene OCR completed, against a fixed 120-second limit. Earlier open-inventory → selected-icon transitions took 4.717 and 4.673 seconds; selected-icon → inventory-evidence transitions took 4.418 and 4.522 seconds. These include input, rendering and scene OCR overhead, not pure per-item CPU time. The offline detail-crop measurement is recorded separately.

The new operational budget is a 120-second base plus 20 seconds for each prospective owned item: known purchases plus the next possible purchase. The 20 seconds conservatively covers roughly twice the observed two-step 9.1-second proof window. The route permits at most one purchase per visit, 12 visits and 12 owned items. The largest visit budget is 360 seconds. Summing the worst-case visit budgets gives 3,000 seconds of pause allowance; the existing 1× campaign base plus that allowance is 4,800 seconds. These remain finite harness limits, not game win conditions or balance changes.

Ownership verification is strengthened: every visible owned icon must expose its exact selected-detail identity, and the count and complete identity set must match all same-run purchases. Partial inspection is retained as evidence but never marked verified. Exact wallet deduction, panel closure, resumed countdown, terminal outcome, seed and normal-exit checks remain required.

This change has no new live campaign proof. See the untouched original report, log, capture timestamps and screenshots in this directory. The new inspection scope and enlarged budget still require a fresh public run.
