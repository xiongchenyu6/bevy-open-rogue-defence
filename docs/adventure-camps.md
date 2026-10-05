# Optional adventure camps

Three camps occupy the north/south battlefield edges, with paths through the road boundary. Camps unlock at 3/8/13 minutes. A camp is a deliberate side objective: preview its boss, guards, abilities, risk and rewards, then take a small gold/wood reward or fight. Preview actors are neutral presentation, not targets or free loot. The first two minutes retain the opening progression.

A challenge sends a distinct ordinary wild leader and three different guards on a one-second parabolic jump from the camp into the road, then activates their actual combat components for up to 90 seconds. Airborne creatures cannot attack or be targeted. The hero keeps their current position. Road waves continue while the hero is away. Wild leaders have distinct charge/lava, tide/undertow and poison/summoning loadouts, with permanent half-health phase changes. The actual camp actors fight the hero and remain independent of the road gate. Exploration remains available during a main boss battle and near its deadline; another open modal, save phase or an already active camp prevents overlapping camp choices.

Boss death alone does not pay: all original guards must also be cleared. Victory pays the displayed gold, wood, XP, one recruitment fruit and one unique basic-attack relic. Safe reward, victory and failure resolve that camp once per expedition. Failure/timeout pays nothing and clears its actors, owned summons and hazards. Hero deaths use normal remaining lives. Optional victories do not count as main boss checkpoints, bank normal encounters or start the final save phase.

## Basic-attack relics

- Cataclysm Volley: every five completed basic attacks, three delayed fire impacts; each deals 150% attack plus 60% spell power within 3.5 units and ignites nearby foes.
- Glacial Shatter: every four basic attacks, a five-unit ice blast deals 180% attack plus 120% spell power and freezes for two seconds, reduced by the existing elite/boss control resistance.
- Plague Cascade: every six basic attacks, infect foes within four units for six seconds, dealing 30% attack plus 20% spell power each second. Uses the existing finite infection propagation (two generations, 32 targets).

Multi-arrow volleys count once, extra genuine basic attacks count independently, and legacy echoes/triggered spells do not recursively advance the relic counters. Relics remain owned through hero absorption and clear at expedition reset. Damage uses the shared spell damage pipeline. Pause freezes casts, particles, damage and control timers.

Validated with strict all-target Clippy, 520 passing library tests (one ignored), native binaries and 12 native scenes / 12,000 frames across English desktop and Chinese mobile. Real pointer clicks during a living main boss, four different wild models, airborne untargetability, landing activation, one-time rewards, all three awarded relic casts and off-axis damage, actual ice freeze, failure/retry and pause cleanup are covered. See [arrival validation receipt](../screenshots/adventure-arrival-validation.txt), [airborne creatures](../screenshots/adventure-boss-0-arrival-final-en-1280x720-arrival-airborne.png), [landed combat](../screenshots/adventure-boss-0-arrival-final-en-1280x720-arrival-landed.png) and [mobile exploration during main boss](../screenshots/adventure-main-boss-arrival-final-zh-667x375-choice.png).

No overall campaign balance, browser performance, natural unlock progression or every aura/ultimate visual verification is implied.
