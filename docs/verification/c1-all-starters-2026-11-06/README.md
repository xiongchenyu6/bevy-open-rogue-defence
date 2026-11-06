# First-chapter stationary-policy sample: 24 completed cases

Frozen simulator source `acbdd2d75c814c6e05ac398f5b3e025dea1b4662`, executable
SHA-256 `cc31f1b4ecf07ebb96dee3c28c9479dec2e9641f72611a9b1b1b876f474fde84`.
The authoritative unit completed with `Result=success`, `ExecMainStatus=0`;
runner child exited 0 after 1233.913 seconds. All 24 games reached actual terminal
states, with no timeouts, frame caps, invalid receipts or stderr ERROR/panic.

The experiment has four requested seeds (hex 1–4), three requested starters and
active/passive stationary policies, chapter index 0, all six permanent upgrades
zero, fixed default behind position, 120000-update cap, jobs 2 and 900 wall seconds
per case. No pressure override or numerical tuning. Actual run seeds are 2–5,
preserved separately from requested seeds. Every first positive-elapsed checkpoint
confirms the intended starter mapping: requested 0 → Stone Warden/actual hero 2,
1 → Ember Ranger/1, 2 → Storm Savant/0. Final heroes can differ after recruitment.

The prior [complete recorded replay match](../post-splash-repeat-2026-11-06/README.md)
checks one BEEF fixture under this same executable. It does not establish global
replay determinism across this sample or all asynchronous asset-loading schedules.

## Observed sample, not a population win-rate estimate

Numeric ranges below are per case. Each passive case has 3 hero deaths
(12 × 3 = 36 total); active cases have 0 deaths each and in total.

| Recorded measure | Active, 12 cases | Passive, 12 cases |
| --- | --- | --- |
| Actual terminal outcome | 12 victories | 12 losses |
| Terminal game elapsed | 1500.596–1521.342 s | 211.077–755.068 s |
| Terminal wave / bosses defeated | 25 / 5 | 4–13 / 0–2 |
| Hero deaths per case | 0 | 3 |
| Learned skill IDs | 8–9 | 1–2 |
| Skill rank sum | 22–28 | 1–4 |
| Skills at/above base rank cap | 0–2 | 0 |
| Gate breaks / exposed time | 0–62 / 0–249.801 s | 2–10 / 6.015–38.339 s |
| Minimum alive health fraction | 0.899819–0.999646 | 0.000207–0.004130 |

Ten active cases first show a base-cap skill at a sampled checkpoint; two never
do. The earliest observed interval is (559.360, 588.992] game seconds. Each case's
own preceding/current checkpoint interval is retained in `results.json` and
`cases.csv`; this is not an exact event timestamp. No passive case observes a
base-cap skill. These recorded builds do not show all learned skills maxed under
this policy, but do not override the user's different real UI/progression route.

All 32 skill definitions have base max rank 5. Ascendant Crown intentionally
raises the highest-ranked skill to at most **6** (`market.rs`, AscendantCrown
purchase effect). Three active cases contain rank 6; their raw vectors are
preserved and valid. “Capped” here means rank ≥ the definition's base max, not
UR evolution/branch completion, occupied-slot capacity or the full unlocked pool.
The verifier permits 0–6 and does not discard or clamp Crown bonuses.

## Policy and interpretation limits

- This is a real-GamePlugin headless controlled fixture, not public keyboard/UI
  acceptance or an earned campaign. Chapter 0 has no synthetic previous clears;
  the manager explicitly supplies chapter/training fixture values and fresh saves.
- Both policies perform compulsory initial and milestone recruitment via card 0.
  Passive means no subsequent skill/bond/market strategy, not literally no input.
  Active greedily chooses draft card 0, prioritizes free/affordable skill draws
  then affordable bonds, and scans market offers 0–4 on its existing schedule.
- Fixed behind position, no movement, subsequent recruitment/absorption and
  different ranges strongly confound requested starter comparisons. Do not rank
  warrior/archer/mage strength from this experiment or treat the initial class as
  the final hero's independent effect. No G/F trial, camp, forge or optimal player
  policy is represented; item ownership and UR branches are not fully telemetered.
- Four seeds are paired across policies/starters, not 24 independent population
  draws. Counts describe these fixtures; no global win rate or causal tuning is
  inferred. Jobs 2 and asynchronous assets remain explicit execution conditions.
- Raw minimum HP changes with maximum-health growth; gate HP 0 is compatible with
  repeated revival. Injured seconds count alive deficit >0.01, not dangerous time.
  Use deaths, alive health fraction, deficit and exposure together.

## Portable evidence and independent verification

`raw/` contains all 24 stdout JSONL and 24 stderr files, losslessly gzipped with
fixed metadata. `raw-manifest.json` retains original paths, uncompressed byte
counts/SHA and compressed SHA; raw decompressed bytes are independently checked.
No raw case log is omitted. `summary.json`, `results.json`, both CSVs and actual
manager/source/binary/frozen-input receipts are retained. The original prepared
README remains historical under `manager/README-prepared.txt`; this completed
report is authoritative.

Run `python3 verify.py` without local target paths, Rust, assets or simulator.
It independently decompresses/parses every case, recomputes outcome/censor,
checks all requested identities, first-live starter/seed/dimension fields,
0–6 ranks, pressure/health/cast metrics, base-cap intervals, final vectors and
both CSVs against raw checkpoints. Actual source/executable provenance is pinned
by archived receipts/raw runner metadata. The 182 MB executable and 191 assets
are **not** duplicated here; this is portable raw-result verification, not a
self-contained simulator rerun package. No plots/environment setup were needed.
