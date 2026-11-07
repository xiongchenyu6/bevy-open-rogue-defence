# Bounded repeat diagnosis

The actual active starter1/seed1 repeat fails. No retry or source change was made after observing the mismatch.

The last common checkpoint at frame37800 is identical in every logged field, including RNG497054159555290102. The frame38088 bond offer and38096 settlement remain equal after removing only root identity. The first recorded nonidentity mismatch is the skill offer at38288, elapsed625.47284. The same recorded rank array, gold0, wood23 and points0 accompany different skill IDs and old-skill mappings. The subsequent blind first-card selection consequently trains different skills.

The next checkpoint39600 has RNG4453497676359710579 in the matrix case and14129337538743026558 in the repeat. Counting the actual resources.rs xorshift transition from37800 reaches those states after915 and882 draws respectively. That interval includes effects of the different selection, so this count does not locate the initial divergent consumer.

Current logs omit picker-entry RNG, anti-drought state, full market equipment state, actor positions/HP and per-system execution order. Identical coarse checkpoints do not establish an identical whole world at the next draw.

One concrete scheduling candidate exists in current source. progression.rs42–64 chains its own Update systems but has no ordering edge against combat::CombatSet. request_skill_draft261 samples the shared RunRng before opening the paused draft. spawn_wave544/760–772, move_enemies1235 and projectile_hits2846/2973 consume the same RNG and check pause state. Skills also have RNG consumers ordered relative to CombatSet, without an edge to the progression chain. Resource access serializes conflicting execution but does not state which chain must run first. balance_sim.rs701 uses a single-threaded executor and still leaves that ordering unspecified. This is a candidate, not a demonstrated cause of this pair.

A future diagnostic could record actual picker-entry/exit RNG, drought flags and level/capacity/selected hero, alongside bounded existing consumer traces around37800–38288. A production-system paired test can place an actual contact and a real G request in the same update under opposite explicitly controlled orderings. Such a test would demonstrate order sensitivity, while actual runtime traces would still be needed to attribute this recorded pair. No instrumentation, test or ordering fix was added in this task.

Raw context and hashes are in repeat-first-difference-context.json. The per-seed sampler tests, paid replacement/training contracts and native UI checks passed independently. Whole-run seed repeatability remains open.
