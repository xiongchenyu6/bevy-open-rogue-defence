# BoneSentry cadence and packaged progression

A controlled always-in-range target received66 outgoing contacts over60simulated seconds at1/60delta,56 at32/60 and28 at32/15. Damage sums were3300/2800/1400. These are emitted SkillStrike outputs, not resolved enemy HP damage. The previous clamp/reset discarded cooldown overshoot.

BoneSentry now preserves fractional cooldown and schedules every due contact while in range. Idle cooldown becomes ready without banking attacks; travel time is excluded from contact time. The same controlled test now produces67contacts/3350damage at all three deltas. Full library tests passed599 cases,0failed,1existingignored; strict Clippy and builds passed. Idle/pursuit regressions passed. Two native summon-assault cases completed1360frames; root inspected4images and actual pursuit/damage/pause/retarget/wall rebuilding checks passed. [Evidence](verification/summon-cadence-2026-10-08/verification.json).

This is a BoneSentry correction, not proof of overall32×equivalence. Other summon lifetimes/attack timers, skill cooldowns and basic-attack backlog still need controlled equal-time checks. The existing Linux candidate source31bdac8 does not yet contain this fix.

A separate real packaged32×UI run (stationary greedy key policy,seedA11CE) lost atlevel21/589kills. It earned34shards, spent15 through the actual Strength upgrade button, saved19shards and one upgrade, restarted with the same progress and exited normally twice. Root inspected8images. No health/currency/win state was injected. The final chapter-clear assertion intentionally failed because victory wasfalse; the job is not a campaign-success pass. Earlier automation retries had wrong duplicate-caption/footer-text clicks and are not accepted evidence. [Failure/progression evidence](verification/packaged-failure-progress-2026-10-08/result.json).

The loss alone does not identify a balance bug or prove its cause was timestep sensitivity; choice timing, strategy and render rate differ from the simulator. Full packaged chapter victory, six-chapter progression and application of the purchased upgrade in a subsequent battle remain unverified.
