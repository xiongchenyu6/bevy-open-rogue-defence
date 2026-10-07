# Skill draft pacing: complete first-chapter audit

Source revision: `b4e4da1`. Immutable simulator snapshot SHA-256: `0bcced1e5b42e82a25e5109ea65d1af376181b04ce7e5ae94a586bc0019a95ec`. The production GamePlugin runs at a fixed step with isolated profiles, zero permanent training, two seeds and three starters. Active chooses the first offered card, buys upgrades and market items; passive does not buy optional upgrades. Both are stationary behind the wall. This is not an optimal player or literal unattended-play guarantee.

All 12 cases finished with valid chapter/training and actual damage-skill telemetry. No capped or timed-out case is counted as a result. Active: 6/6 wins, zero deaths; passive: 0/6 wins, three deaths each.

| Starter | Seed | Learned skills | Mastered skills | Total ranks | Minimum living HP fraction |
|---|---|---:|---:|---:|---:|
| Warrior | a11ce | 9 | 1 | 27 | 0.961 |
| Warrior | beef | 8 | 2 | 27 | 0.845 |
| Archer | a11ce | 9 | 0 | 26 | 0.969 |
| Archer | beef | 8 | 3 | 27 | 0.969 |
| Mage | a11ce | 8 | 1 | 27 | 0.992 |
| Mage | beef | 9 | 2 | 26 | 0.971 |

The previous whole-bar mastery problem is absent in this sample: active builds finish with only 0–3 mastered skills across 8–9 learned skills. However, all active cases still win without a death. First-chapter combat pressure remains light, especially for ranged starts. This verifies draft pacing, not the complete difficulty curve or Steam readiness.

Raw receipts, checkpoint CSVs, analysis summaries and hashes are retained in [the audit directory](balance/draft-pacing-first-2026-10-08/). The sixth-chapter follow-up uses the same binary and remains separate.
