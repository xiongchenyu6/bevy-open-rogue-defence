# Chapter experience pacing

Higher chapters previously multiplied both gold and experience by up to 2. This accelerated in-run attributes and recruitment alongside harder enemies: all six untrained active sixth-chapter baseline samples reached level 60 and won without a death.

Chapter scaling now increases enemy health, enemy damage and gold, while the same enemy gives the same experience. `Chapter.gold_scale` replaces the misleading combined reward field. First-chapter rules are unchanged. This preserves higher-stage income without automatically counteracting difficulty through faster level growth.

## Complete production-system checks

Two seeds × three starter classes, stationary behind the wall, first-card greedy upgrades, isolated profiles and immutable binary snapshots. Every result below finished, passed chapter/training checks and supplied actual damage-skill telemetry.

| Scenario | Wins | Final levels | Notes |
|---|---:|---|---|
| Old sixth chapter, no permanent training | 6/6 | 60 | No hero deaths |
| New first chapter, no permanent training | 6/6 | 42–44 | No hero deaths |
| New second chapter, no permanent training | 6/6 | 43–45 | No deaths; lowest living HP 68.19% |
| New third chapter, no permanent training | 6/6 | 42–44 | No deaths; lowest living HP 72.01% |
| New fourth chapter, no permanent training | 6/6 | 43–44 | One life lost across six cases; lowest living HP 0.0021% |
| New fifth chapter, no permanent training | 6/6 | 43–45 | One life lost across six cases; lowest living HP 0.0014% |
| New sixth chapter, no permanent training | 4/6 | 43–44 for wins | Warrior BEEF loses at level 9; mage BEEF loses at level 19 |
| New sixth chapter, all permanent upgrades at level 5 | 6/6 | See receipts | Warrior BEEF uses one life; some other builds reach critically low health |

Permanent training changes kills, draw timing and later choices, so health minima are not monotonic per seed. The result supports meaningful progression in this small sample; it does not establish an unconditional win rate or optimal-player difficulty. Level-5 training is a direct fixture, not proof of an earned campaign purchase path. Its six-stat cost is 1,950 shards under the current production price formula.

The library suite passes 579 tests (0 failed, 1 existing ignored), strict all-target Clippy passes, and the simulator and desktop binary build. Raw candidate/trained logs and analysis are in [the verification directory](balance/chapter-xp-pacing-2026-10-08/); old sixth-chapter logs are in [the baseline directory](balance/draft-pacing-sixth-before-xp-2026-10-08/). The 24 middle-chapter cases completed without truncation or invalid receipts; raw logs and 1,319 checkpoints are in [the middle-chapter verification directory](balance/chapter-xp-middle-2026-10-08/). Early chapters remain forgiving under this active policy; fourth/fifth-chapter victories include actual life loss and near-death pressure. This small sample does not prove class balance, optimal-player difficulty or all builds. A real earned-upgrade playthrough, more seeds, all heroes/routes and optional encounters still require verification. Steam readiness remains unproven.
