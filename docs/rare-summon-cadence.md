# Finite summon cadence

Four real pooled summon fixtures use an8-second lifetime and10seconds of total simulated time. Targets remain alive and stationary; casting, projectile movement and collision are excluded. Each step counts actual newProjectile/DamageProfile orSkillStrike output, returns projectiles to their pool, finalizes recycling, and verifies no residual output. Expired summons must loseSkillSummon through their real pool. Nominal damage is50per output and is not resolved enemy HP damage.

| Summon | Before1/60 | Before32/60 | Before32/15 | Before10-second step | After1/60 | After32/60 | After32/15 | After10-second step |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Tide elemental |8|8|3|0|8|9|9|9|
| Bolt turret |11|8|3|0|12|12|12|12|
| Frost wyrm |7|5|3|0|7|7|7|7|
| Ancestor colossus |3|3|2|0|3|3|3|3|

The correction retains cooldown remainder, limits movement and attacks to living time, and excludes travel time from in-range attacks. Idle time only makes the weapon ready, without banking hits. Gate downtime still freezes turret cooldown while consuming lifetime. Every successful frost projectile still advances its control counter. Colossus pull uses living time. All exit paths retire expired summons after the final valid behavior.

Tide retains a one-shot endpoint discrepancy across float steps; the regression allows one contact/50nominal damage difference and does not establish exact replay or whole-game32×equivalence. Independent idle and pursuit tests passed. The unchanged turret gate-down/resume regression passed. Final library tests passed604cases,0failed,1existingignored; strict all-targetClippy passed. Both binary builds passed. Two native summon-assault cases completed1360frames at1280×720 and667×375; four screenshots were inspected. Actual pursuit/damage/pause/retarget/outside-wall formation and finite-health wall rebuild checks passed. [Evidence](verification/rare-summon-cadence-2026-10-09/verification.json).

The first two baseline attempts failed to compile the new fixture (profile index type andFnOnce closure), so only the third executable run establishes the failing gameplay baseline. Full campaign wins, other skill timers, final status-control cadence, real-time performance and refreshed packaging remain outside this verification.
