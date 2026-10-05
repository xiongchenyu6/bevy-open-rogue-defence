# Boss battles

Five bosses retain their original movement, healing, summon and stance mechanics and add distinct target-locked hazards. Warning geometry is the authoritative damage geometry, with no invisible expanded hit region.

| Boss | Primary hazard | Alternating hazard |
| --- | --- | --- |
| Cinder Colossus | Charge strip, half-width 1.25, length 7 | Lava circle, radius 2.6, duration 3 s |
| Abyssal Leviathan | Tidal strip, half-width 2.2, length 16 | Undertow circle, radius 3.4, duration 2 s |
| Dread Veil | Locked strike, radius 1.7 | Three locked marks; five in phase two |
| Sun Crown Magus | Ray strip, half-width 0.8, length 20 | Two rays; three in phase two |
| Green Regent | Summons plus poison circle, radius 2.8 | Poison duration 4 s, with continued summons |

First crossing 50% health permanently enters phase two: cast frequency increases 30%, warning and hit widths/radii increase 25%, and hazard damage changes from 1.2 to 1.6 times base attack. Healing cannot revert the phase. Existing timed final-boss enrage remains.

Warnings lock the hero's location before activation. Circle and forward-strip boundaries use the same values for presentation and collision. Charge and shadow strikes deal physical damage through HeroVitals armor; lava, tides, rays and poison use magical resistance. Invulnerability blocks damage and tidal stun. A live gate intercepts hazards from outside and protects the hero behind it; intersection covers the whole road-blocking wall, including off-center attacks. Persistent areas tick every 0.5 seconds; tidal hits briefly stun. Battle pause/save freezes timers. Dead bosses and finished battles remove hazards even when paused.

Verification evidence is recorded separately after native desktop/mobile captures and behavioral tests. No browser or broad campaign balance claim follows from these checks.

Final receipts: [native validation](../screenshots/boss-pressure-validation.txt) and [bounded before/after simulation data](../screenshots/raid-balance-comparison.json). Strict all-target Clippy and 497 library tests passed (one ignored). Thirty-two desktop/mobile scenarios exercised boss hazards, reinforcements, concentrated waves and all eight elite roles. Across three 45,000-frame before/after runs, lifetime live-enemy peaks changed from 20/16/15 to 37/35/41, with no enemy-pool spawn rejections. These runs reached wave 23 and four bosses, not a complete campaign victory. The strong autoplay build retained mostly healthy gates, so increased crowd pressure does not establish balanced difficulty. A repeated seed matched the first fourteen checkpoints, then diverged in RNG at frame 25,200 and later gameplay; exact replay determinism remains unverified.

## Concentrated waves

The first two minutes retain the tutorial cadence. From wave three, roughly 40% of the original per-minute budget arrives as patrols; the remainder arrives as two concentrated groups at 20 and 44 seconds. Each group has a three-second portal warning. Wave four onward promotes the first actor of each group to an elite and gradually increases additional elite chances from 2.5% to a 9% ceiling. These promotions do not add to the original 1,650 ordinary-enemy budget. Pool pressure retains pending budgets rather than dropping spawns; existing backlog merging remains bounded.

## Elite pressure

Reference: the local `protect-carrot` monster/enemy implementation. The six reference roles join the existing Volatile and Commander roles, with eight distinct overhead crests and active aura presentation.

| Affix | Actual role |
| --- | --- |
| Frenzy / 狂乱 | Faster movement and attacks |
| Bulwark / 硬壳 | Armor and an intermittent shield |
| Yellow Sign / 黄印 | Within 4.5 units, learned skills and automatic ultimates recover cooldown at half speed; leaving restores normal recovery |
| Brood / 孵化 | Death releases three ordinary offspring; offspring do not inherit the affix |
| Bloodrite / 血祭 | Regenerates 1.2% of its own maximum health per second and heals nearby allies within 3.5 units by a fixed wave-scaled amount; overlapping healers do not stack. Retains one revival at 35% health |
| Siege / 攻城 | Prioritizes the gate, attacks from range, and doubles gate damage |
| Volatile / 爆裂 | A delayed death explosion with a warning |
| Commander / 统御 | Boosts nearby allies' health, damage and action speed by 20%; matching auras do not stack |

Yellow Sign suppresses cooldown recovery, rather than preventing skill casting. Bloodrite does not heal bosses for a percentage of boss maximum health.

## Boss reinforcements

Dread Veil summons three attackers in phase one and six in phase two. Green Regent summons five and ten respectively. Each boss can own at most twelve living reinforcements; later casts fill only available slots. Clearing reinforcements opens space for the next cast. Summon casts blocked by a full global pool retry briefly, then release the cast instead of freezing the boss. Surviving reinforcements keep fighting after their boss dies, but a dead boss cannot summon again.
