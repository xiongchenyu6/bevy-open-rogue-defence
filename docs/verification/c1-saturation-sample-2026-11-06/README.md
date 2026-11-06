# First-chapter progression sample: four seeds, two fixed policies

All eight cases actually finished without a frame cap, timeout, invalid receipt
or input-blocked outcome. This is a **descriptive sample**, not general win rate,
causal tuning, cross-process determinism, all-class acceptance or player testing.
No balance values were changed. The same frozen release GamePlugin simulator and
191 read-only asset files were used throughout. Binary SHA-256:
`558f626d5c5055914296bfe1ece2a0f409493c42bce5323919cbe52bd7997996`, source
`7e87900e2eb59dfe91de0a0020d49dd1656e4e2d`.

The [same binary's full repeat still differs](../post-gate-cinder-repeat-2026-11-06/README.md).
That negative is retained; these different-seed observations do not supersede it.
The eight cases ran serially after that job was authoritatively terminal.

## Actual observations

| Requested seed | Active end / game seconds | Deaths | Learned skills | At/above base rank cap | First observed base cap interval (seconds) | Minimum alive HP fraction |
| --- | --- | ---: | ---: | ---: | --- | ---: |
| 1 | victory / 1514.113 | 0 | 8 | 1 | (647.972,676.851] | 0.943664 |
| 2 | victory / 1500.913 | 0 | 9 | 1 | (1258.438,1287.014] | 0.967472 |
| 3 | victory / 1500.657 | 0 | 9 | 1 | (1025.452,1055.012] | 0.996447 |
| 4 | victory / 1511.625 | 0 | 9 | 2 | (559.360,588.992] | 0.982469 |

| Requested seed | Recruitment-only end / game seconds | Deaths | Learned skills | At/above base rank cap |
| --- | --- | ---: | ---: | ---: |
| 1 | defeat / 360.769 | 3 | 1 | 0 |
| 2 | defeat / 534.478 | 3 | 2 | 0 |
| 3 | defeat / 325.406 | 3 | 1 | 0 |
| 4 | defeat / 755.068 | 3 | 2 | 0 |

Actual recorded run seeds are requested seed plus one here (2/3/4/5); exact
checkpoints preserve them. Recruitment changes the final hero, so requested
starter 2 must not be treated as final hero ID 2.

The active samples did not finish with all learned skills at their base cap.
Their learned sets differ; pairwise Jaccard overlap is 0.133 to 0.417 among these
four samples. This demonstrates these particular different builds, not adequate
variety for the full pool. The active samples also show very low health pressure:
zero deaths and minimum alive health fractions 94.4% to 99.6%. This is a concrete
reason to examine later chapters/classes and pressure sources, rather than
announcing overall balance success or blindly increasing skill rewards.

`curves.csv` preserves every sampled checkpoint's game time, level, learned
count, capped count, rank total and owned bond-card count. First-cap intervals
use the preceding/current checkpoint; they are **not exact event timestamps**.
The actual 32 skill definitions have base max rank 5. Some recorded ranks are 6;
counts use at-or-above 5 and do not assert completion of evolution branches.
No skill-slot capacity or whole-pool saturation is inferred. Owned bond-card
count is not completed-combination count.

## Policy and fixture boundaries

Both policies use chapter index 0, no prior clears, zero permanent upgrades,
fresh isolated saves, requested starter portrait 2 (Storm), default behind
position, fixed 1/60 updates and single-thread schedules. Rendering/window/audio
output are disabled, while asynchronous asset/scene loading remains. There is no
pressure override. The frame-one progress setup is a controlled fixture, not
an earned save or a UI acceptance flow.

Active chooses card zero via real selection handlers, spends real skill/bond
currency, and from 180 seconds scans the five real market offers every 60 game
seconds. Recruitment-only still selects a starter and compulsory level 10/20/40
recruitment cards; it is **not a completely input-free player**. Neither policy
moves, chooses counters intelligently, runs optional camps, performs forge
recipes, or explores a build optimizer. Policy decisions occur every eight
updates, not at a guaranteed game-time decision cadence. Four seeds and one
requested starter do not cover every profession, chapter, strategy or campaign.

## Receipts and reproducibility

Complete raw JSONL/stderr files, final receipts, derived curves and exact input
manifest are archived. Frozen binary/assets are hashed but not stored in Git.
`verify.py` recomputes the reported learned/base-cap metrics from archived raw
checkpoints and checks all archive hashes; it does not rerun the simulator.

The first launch failed in preflight before starting any case because a completed
transient systemd unit had cleared InvocationID. Its script/log are retained.
The corrected preflight requires terminal inactive/success/exit-zero state and
all four prior concrete process receipts, rejects a different reported ID, and
accepts the cleared ID of an ended transient unit. The second launch ran the
single eight-case batch; no case was restarted to replace an unfavorable result.
