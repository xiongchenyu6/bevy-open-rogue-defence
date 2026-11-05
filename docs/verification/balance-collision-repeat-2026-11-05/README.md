# Current-source duplicate replay gate: failed

Two complete, uncensored runs used the **same frozen executable** (`be543db9165e554f2b5fc9837721f3361ac4593213f0b1125bf8c4d690a423a3`), seed `A11CE`, chapter index 5 (chapter six), no permanent upgrades, starter policy 2 (Storm Savant), stationary behind-gate positioning and the active greedy policy. The 120000-frame cap and 900-second subprocess limit were not reached. Both processes exited normally with valid chapter/upgrade/cast-telemetry receipts.

| Run | Result | Terminal frame | Actual battle seconds | Deaths | Kills | Wall seconds |
| --- | --- | --- | --- | --- | --- | --- |
| A | Loss | 57127 | 933.754 | 3 | 830 | 328.69 |
| B | Win | 97908 | 1539.182 | 1 | 2164 | 528.51 |

The first **18 recorded checkpoint objects** are identical in all fields, from frame 32 through frame 30600. The first recorded difference is zero-based checkpoint index 18, frame **32400**: RNG state, owned bonds, gold, wood, kills, attacks, enemy peak and elapsed/injury time differ. Elapsed time is 530.073 versus 530.010 seconds. This bounds the first *recorded* difference; it does not establish when internal state first diverged.

[Strict comparison](comparison.json) exits 1. [Full-field comparison](full-checkpoint-comparison.json) also rejects equality. Both normal terminal receipts are retained under [A](replay-a/summary.json) and [B](replay-b/summary.json), together with original JSONL and stderr. [Analysis](analysis/cases.csv) and [checkpoint curves](analysis/curves-30s.csv) contain 89 checkpoints across these two runs. They are not population win-rate estimates.

The simulator uses the production GamePlugin and real authoritative combat/progression systems, fixed 1/60-second time and single-threaded schedules, with no renderer or physical input. The active policy picks card zero, handles later recruitment and tries market offers; it does not represent optimal human play, earned chapter access, side quests or a packaged UI run. Chapter access/prior clears and zero upgrade levels are synthetic profile initialization. Skill/bond affordability now uses the production price helpers. The separately proven [stationary collision ordering fix](../collision-order-2026-11-05/README.md) is included, but these results **do not identify the cause** of the remaining full-run divergence.

The manager [run.py](run.py) waits for the prior Cargo owner, builds once and copies one snapshot for both runs. [Binary receipt](binary.json), identical before/after-build source manifests, build/runner logs and hashes are archived. The build began at HEAD `6995368` with documented UI changes; [source reconstruction](source-reconstruction.json) confirms every captured Cargo/Rust source digest matches clean commit `49562e6`. This is source identity evidence, not a complete frozen external-asset archive. The executable remains in the target verification directory; the large binary is not committed here.

The **24-case matrix remains withheld**. No numerical tuning was performed. Next work is a targeted per-frame trace and regression for a demonstrated cause, followed by another duplicate gate. A deterministic micro-regression passing does not establish whole-run determinism.
