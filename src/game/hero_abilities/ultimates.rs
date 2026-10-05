//! Automatic hero ultimates and their gameplay sequences.
use super::*;

pub(super) fn observe_ultimate_cast(
    mut commands: Commands,
    selected: Res<SelectedHero>,
    mut vitals: ResMut<HeroVitals>,
    mut active: ResMut<ActiveSkill>,
    mut cues: ResMut<UltimateCues>,
    assets: Option<Res<GameAssets>>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    market_effects: Res<MarketEffects>,
    hero: Query<&Transform, With<Hero>>,
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
    player_summons: Query<(), With<PlayerSummon>>,
    mut enemies: Query<
        (
            Entity,
            &mut Enemy,
            &Transform,
            Option<&BossEnemy>,
            Option<&TimedSpeedEffect>,
            Option<&EliteAffix>,
        ),
        With<Enemy>,
    >,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if runtime.ultimate_gap > 0.0
        || runtime.sequence.is_some()
        || !selected.locked
        || session.paused
        || session.finished
        || !vitals.can_act()
    {
        return;
    }
    let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) else {
        return;
    };
    let target = enemies
        .iter()
        .filter(|(_, enemy, _, _, _, affix)| {
            enemy.hp > 0.0 && !affix.is_some_and(|a| a.is_volatile_pending())
        })
        .min_by(|a, b| {
            hero.translation
                .distance_squared(a.2.translation)
                .total_cmp(&hero.translation.distance_squared(b.2.translation))
        })
        .map(|(_, _, transform, _, _, _)| transform.translation);
    let available = player_summon_slots(
        player_summons
            .iter()
            .count()
            .max(pools.summon_active_count()),
        market_effects.summon_cap_bonus(),
    );
    let useful = |kind| {
        automatic_ultimate_useful(
            kind,
            hero.translation,
            target,
            session.core_hp < session.core_max_hp * 0.85,
            available,
        )
    };
    let kind = if active.remaining <= 0.0 && useful(selected.kind) {
        Some(selected.kind)
    } else {
        (0..HeroKind::COUNT)
            .map(|offset| HeroKind::ALL[(runtime.ultimate_cursor + offset) % HeroKind::COUNT])
            .find(|kind| {
                *kind != selected.kind
                    && runtime.inherited[kind.index()]
                    && runtime.inherited_ultimate_remaining[kind.index()] <= 0.0
                    && useful(*kind)
            })
    };
    let Some(kind) = kind else { return };
    let target = target.unwrap_or(hero.translation);
    if kind == selected.kind {
        active.remaining = active.cooldown;
    }
    runtime.inherited_ultimate_remaining[kind.index()] = effective_ultimate_cooldown(kind);
    runtime.ultimate_gap = 3.;
    runtime.ultimate_cursor = (kind.index() + 1) % HeroKind::COUNT;
    let cast_origin = if kind == HeroKind::GroveArtificer {
        core.single().map_or(hero.translation, |c| c.translation)
    } else {
        hero.translation
    };
    let cast_target = if kind == HeroKind::StarWagerer {
        match choose_fate_outcome(
            session.core_hp / session.core_max_hp.max(1.0),
            enemies.iter().count(),
        ) {
            FateOutcome::Damage => enemies
                .iter()
                .min_by(|a, b| {
                    b.3.is_some().cmp(&a.3.is_some()).then_with(|| {
                        hero.translation
                            .distance_squared(a.2.translation)
                            .total_cmp(&hero.translation.distance_squared(b.2.translation))
                    })
                })
                .map_or(target, |e| e.2.translation),
            _ => hero.translation,
        }
    } else {
        target
    };
    cues.push(kind, cast_origin, cast_target, UltimatePhase::Cast);
    let ability_scale = HEROES[kind.index()].rarity.hero_ability_multiplier();
    let control_scale = HEROES[kind.index()].rarity.hero_stat_multiplier();

    match kind {
        HeroKind::StormSavant => {
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 10,
                pulse_timer: 0.0,
                pulse_interval: 0.6,
            });
        }
        HeroKind::EmberRanger => {
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 5,
                pulse_timer: 0.0,
                pulse_interval: 0.34,
            });
        }
        HeroKind::StoneWarden => {
            for (entity, _enemy, transform, _, speed_effect, _) in &mut enemies {
                if hero.translation.distance(transform.translation) > 7.5 {
                    continue;
                }
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    0.28,
                    6.0 * control_scale,
                ));
            }
            session.heal_gate(90.0 * ability_scale);
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 6,
                pulse_timer: 0.0,
                pulse_interval: 1.0,
            });
        }
        HeroKind::SpiritKeeper => {
            let available = player_summon_slots(
                player_summons
                    .iter()
                    .count()
                    .max(pools.summon_active_count()),
                market_effects.summon_cap_bonus(),
            );
            for spirit in 0..6.min(available) {
                let angle = spirit as f32 / 6.0 * std::f32::consts::TAU;
                spawn_spirit(
                    &mut commands,
                    &mut pools,
                    assets,
                    hero.translation,
                    angle,
                    2.0,
                    Some(12.0),
                    0.72,
                    false,
                );
            }
        }
        HeroKind::TideWalker => {
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 3,
                pulse_timer: 0.0,
                pulse_interval: 0.55,
            });
        }
        HeroKind::StarWagerer => {
            let enemy_count = enemies.iter().count();
            let core_ratio = session.core_hp / session.core_max_hp.max(1.0);
            match choose_fate_outcome(core_ratio, enemy_count) {
                FateOutcome::Damage => {
                    let target = enemies
                        .iter()
                        .map(|(_, _, transform, boss, _, _)| {
                            (
                                transform.translation,
                                boss.is_some(),
                                hero.translation.distance_squared(transform.translation),
                            )
                        })
                        .min_by(|left, right| {
                            right.1.cmp(&left.1).then_with(|| {
                                left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal)
                            })
                        })
                        .map(|(position, _, _)| position)
                        .unwrap_or(hero.translation);
                    spawn_ultimate_zone(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        target,
                        build.attack_damage * 6.0 + session.level as f32 * 20.0,
                        7.0,
                    );
                }
                FateOutcome::Healing => {
                    let repair = (240.0 + session.level as f32 * 10.0) * ability_scale;
                    session.heal_gate(repair);
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        hero.translation,
                        5.5,
                        0.8,
                    );
                }
                FateOutcome::Resources => {
                    session.gold = session
                        .gold
                        .saturating_add((180.0 * ability_scale).round() as u32);
                    session.wood = session
                        .wood
                        .saturating_add((15.0 * ability_scale).round() as u32);
                    session.kills = session
                        .kills
                        .saturating_add((30.0 * ability_scale).round() as u32);
                    spawn_ability_fx(
                        &mut commands,
                        &mut pools,
                        assets,
                        HeroKind::StarWagerer,
                        hero.translation,
                        3.4,
                        0.8,
                    );
                }
            }
        }
        HeroKind::FrostOracle => {
            for (entity, _enemy, _, boss, speed_effect, _) in &mut enemies {
                let (multiplier, remaining) = if boss.is_some() {
                    (0.5, 4.0)
                } else {
                    (0.0, 2.5)
                };
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    multiplier,
                    remaining * control_scale,
                ));
            }
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 4,
                pulse_timer: 0.0,
                pulse_interval: 0.55,
            });
        }
        HeroKind::IronBerserker => {
            let missing_health = (vitals.max_health - vitals.health).max(0.0);
            vitals.health =
                (vitals.health + missing_health * 0.25 * ability_scale).min(vitals.max_health);
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 6,
                pulse_timer: 0.0,
                pulse_interval: 0.32,
            });
        }
        HeroKind::RiftDancer => {
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 8,
                pulse_timer: 0.0,
                pulse_interval: 0.22,
            });
        }
        HeroKind::GroveArtificer => {
            session.heal_gate(160.0 * ability_scale);
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 6,
                pulse_timer: 0.0,
                pulse_interval: 0.75,
            });
        }
        HeroKind::SolarVanguard => {
            session.heal_gate(120.0 * ability_scale);
            vitals.health =
                (vitals.health + vitals.max_health * 0.25 * ability_scale).min(vitals.max_health);
            for (entity, _enemy, _, _, speed_effect, _) in &mut enemies {
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    0.75,
                    2.0 * control_scale,
                ));
            }
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 5,
                pulse_timer: 0.0,
                pulse_interval: 0.6,
            });
        }
        HeroKind::VoidReaper => {
            for (entity, _enemy, _, _, speed_effect, _) in &mut enemies {
                commands.entity(entity).insert(merged_speed_effect(
                    speed_effect,
                    0.55,
                    4.0 * control_scale,
                ));
            }
            runtime.sequence = Some(UltimateSequence {
                kind,
                target,
                pulses_left: 5,
                pulse_timer: 0.0,
                pulse_interval: 0.5,
            });
        }
    }
}

pub(super) fn tick_ultimate_sequence(
    mut commands: Commands,
    mut cues: ResMut<UltimateCues>,
    time: Res<Time>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    build: Res<PlayerBuild>,
    assets: Option<Res<GameAssets>>,
    hero: Query<&Transform, With<Hero>>,
    core: Query<&Transform, (With<Core>, Without<Hero>)>,
    enemies: Query<(&Transform, Option<&BossEnemy>), With<Enemy>>,
    targets: Query<
        (
            Entity,
            &Transform,
            Option<&BossEnemy>,
            Option<&TimedSpeedEffect>,
        ),
        With<Enemy>,
    >,
    mut runtime: ResMut<HeroAbilityRuntime>,
    mut pools: ResMut<CombatPools>,
) {
    if session.paused || session.finished {
        return;
    }
    if !vitals.can_act() {
        runtime.sequence = None;
        return;
    }
    let Some(mut sequence) = runtime.sequence else {
        return;
    };
    let (Some(assets), Ok(hero)) = (assets.as_deref(), hero.single()) else {
        return;
    };

    sequence.pulse_timer -= time.delta_secs();
    if sequence.pulse_timer > 0.0 {
        runtime.sequence = Some(sequence);
        return;
    }

    let effect_target = match sequence.kind {
        HeroKind::StoneWarden | HeroKind::IronBerserker => hero.translation,
        HeroKind::VoidReaper => sequence.target,
        HeroKind::TideWalker => hero
            .translation
            .lerp(sequence.target, (4 - sequence.pulses_left) as f32 / 3.0),
        _ => preferred_target(Some(hero), &enemies).unwrap_or(sequence.target),
    };
    let effect_origin = if sequence.kind == HeroKind::GroveArtificer {
        core.single().map_or(hero.translation, |c| c.translation)
    } else {
        hero.translation
    };
    cues.push(
        sequence.kind,
        effect_origin,
        effect_target,
        UltimatePhase::Pulse,
    );
    match sequence.kind {
        HeroKind::StormSavant => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_ultimate_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    45.0 + session.level as f32 * 5.0 + build.attack_damage * 0.45,
                    2.5,
                );
            }
        }
        HeroKind::EmberRanger => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_projectile_fan(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    hero.translation,
                    target,
                    5,
                    build.attack_damage * 0.72,
                    &build,
                );
            }
        }
        HeroKind::StoneWarden => {}
        HeroKind::TideWalker => {
            spawn_ultimate_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                effect_target,
                build.attack_damage * 0.9,
                3.5,
            );
            for (entity, transform, boss, speed) in &targets {
                if transform.translation.distance(effect_target) <= 3.5 {
                    commands.entity(entity).insert((
                        WetMark { remaining: 8.0 },
                        merged_speed_effect(speed, if boss.is_some() { 0.55 } else { 0.25 }, 3.0),
                    ));
                }
            }
        }
        HeroKind::FrostOracle => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_ultimate_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    build.attack_damage * 0.6 + session.level as f32 * 5.0,
                    3.0,
                );
            }
        }
        HeroKind::IronBerserker => {
            spawn_ultimate_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                hero.translation,
                build.attack_damage * 0.75,
                2.8,
            );
        }
        HeroKind::RiftDancer => {
            spawn_ultimate_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                effect_target,
                build.attack_damage * 1.15,
                1.6,
            );
        }
        HeroKind::GroveArtificer => {
            let center = effect_target;
            spawn_ultimate_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                center,
                build.attack_damage * 0.65,
                5.0,
            );
        }
        HeroKind::SolarVanguard => {
            if let Some(target) = preferred_target(Some(hero), &enemies) {
                spawn_ultimate_zone(
                    &mut commands,
                    &mut pools,
                    assets,
                    sequence.kind,
                    target,
                    build.attack_damage * 0.8,
                    3.2,
                );
            }
        }
        HeroKind::VoidReaper => {
            for (entity, transform, boss, _) in &targets {
                if transform.translation.distance(sequence.target) <= 6.0 {
                    commands.entity(entity).insert(VoidPull {
                        center: sequence.target,
                        remaining: 0.5,
                        speed: if boss.is_some() { 0.8 } else { 3.0 },
                    });
                }
            }
            spawn_ultimate_zone(
                &mut commands,
                &mut pools,
                assets,
                sequence.kind,
                sequence.target,
                build.attack_damage * 1.15,
                4.0,
            );
        }
        HeroKind::SpiritKeeper | HeroKind::StarWagerer => {}
    }

    sequence.pulses_left = sequence.pulses_left.saturating_sub(1);
    sequence.pulse_timer += sequence.pulse_interval;
    runtime.sequence = (sequence.pulses_left > 0).then_some(sequence);
    if runtime.sequence.is_none() {
        runtime.ultimate_gap = runtime.ultimate_gap.max(3.);
    }
}

pub(super) fn automatic_ultimate_useful(
    kind: HeroKind,
    hero: Vec3,
    target: Option<Vec3>,
    core_hurt: bool,
    summon_slots: usize,
) -> bool {
    match kind {
        HeroKind::StoneWarden => core_hurt || target.is_some_and(|t| t.distance(hero) <= 7.5),
        HeroKind::IronBerserker => target.is_some_and(|t| t.distance(hero) <= 2.8),
        HeroKind::SpiritKeeper => target.is_some() && summon_slots > 0,
        HeroKind::GroveArtificer | HeroKind::SolarVanguard | HeroKind::StarWagerer => {
            target.is_some() || core_hurt
        }
        _ => target.is_some(),
    }
}
