# Skill draft variety and purchase pacing

Skill hands previously guaranteed the highest-ranked owned skill in the first position. Purchases stopped increasing in price at 260 gold. Together, these rules let players repeatedly buy predictable upgrades and master the whole equipped bar.

The revised design keeps seeded rarity-weighted draws, unique candidates, the one-UR-per-hand limit, and preference drought protection. It removes the highest-rank guarantee. Eligible non-UR ascensions compete with ordinary candidates instead of always displacing them in skill-ID order. If several UR skills await branch selection, the skill offered is selected randomly; its three evolution paths still remain an explicit choice. UR replacement cards have a 5% chance per hand when ordinary upgrades or ascensions remain; an exhausted training pool still offers replacements so it has a growth outlet.

Paid draw cost uses the number of previous paid draws, `n`:

```text
80 + 15n + 20 × max(n - 2, 0)² gold
```

The first three purchases remain 80, 95, and 110 gold. Later purchases continue increasing rather than capping. Arithmetic saturates at the currency representation limit. Free draws remain unchanged and are consumed first. Continuous card selection still stops at insufficient funds; refresh and choosing later preserve their existing payment rules.

## Verification scope

The budget audit uses the real progression plugin, generated UI buttons, and actual card clicks across 32 seeds per budget. Each case starts with an empty six-slot skill bar at level 30, ten minutes, with either 5,000 or 10,000 gold; it receives the normal free grants and favors the strongest offered skill. It records total selections, mastered skills, six-skill mastery, and distinct final rank arrays.

Observed results across the same 32 seeds and strongest-offered-skill policy:

| Gold budget | Mean selections before → after | Mean mastered skills before → after | Six mastered skills before → after |
|---|---:|---:|---:|
| 5,000 | 29 → 16 | 5 → 1.75 | 0/32 → 0/32 |
| 10,000 | 30 → 19 | 6 → 2.78 | 32/32 → 0/32 |

Both versions produced 32 distinct rank arrays in each budget group. This is evidence of slower mastery, not evidence that the count of distinct builds increased. Separate distribution tests verify that the highest-ranked skill can be absent, first-card identities vary, all eligible ascensions can appear, and UR replacements are rare while ordinary training remains.

This controlled audit measures draft and purchase behavior. It does not simulate earning that budget, fighting enemies, optional challenges, or complete campaign win rates. Full combat balance remains a separate verification requirement.

The isolated draft revision passed all 572 library tests (0 failed, 1 existing ignored). The combined final revision, including audio recovery, passed 575 library tests. Before/after draft audit logs are retained in `verification/skill-draft-variety-2026-10-07/`; final dependency checks are in `verification/audio-device-recovery-2026-10-07/`.
