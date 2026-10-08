//! Real zone outputs and pooled banner repairs across finite lifetime steps.
use super::*;
use bevy::ecs::system::RunSystemOnce;
use std::time::Duration;

fn fixture() -> App {
    let mut app = App::new();
    app.init_resource::<Time>()
        .init_resource::<GameSession>()
        .init_resource::<PlayerBuild>()
        .init_resource::<HeroVitals>()
        .init_resource::<SelectedHero>()
        .init_resource::<CombatPools>()
        .init_resource::<SkillRuntime>();
    app.world_mut().resource_mut::<PlayerBuild>().skills[21] = 1;
    {
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.core_max_hp = 100.0;
        session.core_hp = 50.0;
        session.gate_revive_remaining = 0.0;
    }
    // Keep system change ticks alive; a fresh run_system_once would see
    // SelectedHero's insertion on every invocation and immediately retire banners.
    app.add_systems(Update, (update_spell_zones, update_oath_banners).chain());
    app.update();
    app
}

fn advance(app: &mut App, seconds: f32) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_secs_f32(seconds));
}

fn enemy(app: &mut App, position: Vec3) -> Entity {
    app.world_mut()
        .spawn((
            Enemy {
                hp: 1000.0,
                max_hp: 1000.0,
                speed: 1.0,
                damage: 1.0,
                xp: 1,
                wood: 0,
                gold: 1,
                kill_credit: 1,
                elite: false,
            },
            Transform::from_translation(position),
        ))
        .id()
}

fn zone(app: &mut App, kind: ZoneKind, remaining: f32, cooldown: f32) -> Entity {
    app.world_mut()
        .spawn((
            SkillZone {
                kind,
                remaining,
                pulse_remaining: cooldown,
                damage_per_pulse: 7.0,
                radius: 10.0,
                glacier_hits: HashMap::new(),
                profile: DamageProfile::for_skill(0),
            },
            Transform::default(),
        ))
        .id()
}

fn banner(app: &mut App, remaining: f32, cooldown: f32) -> Entity {
    app.world_mut()
        .run_system_once(
            move |mut commands: Commands, mut pools: ResMut<CombatPools>| {
                let entity = pools
                    .rent_mesh_summon(
                        &mut commands,
                        SummonVisual::OathBanner,
                        Handle::default(),
                        Handle::default(),
                        Transform::default(),
                    )
                    .unwrap();
                commands.entity(entity).insert((
                    SkillSummon,
                    OathBanner {
                        remaining,
                        repair_remaining: cooldown,
                        damage_reduction: 0.22,
                        applied: false,
                        armor_bonus: 0.0,
                        attack_speed_bonus: 0.0,
                    },
                ));
                entity
            },
        )
        .unwrap()
}

fn strike_count(app: &mut App) -> usize {
    app.world_mut()
        .query::<&SkillStrike>()
        .iter(app.world())
        .count()
}

#[test]
fn expiry_outputs_are_independent_of_frame_size() {
    // Same 3.2 seconds, 1.4 seconds of remaining life, first effect already due in .15s.
    // Storm: .15/.75/1.35; glacier: .15/.95; vortex: .15/.65/1.15; repairs: .15/1.15.
    let mut discrepancies = Vec::new();
    for delta in [1.0_f32 / 60.0, 32.0 / 60.0, 32.0 / 15.0, 3.2] {
        for (kind, expected) in [
            (ZoneKind::StormMatrix, 3),
            (ZoneKind::GlacierCore, 2),
            (ZoneKind::VortexColumn, 3),
        ] {
            let mut app = fixture();
            let target = enemy(&mut app, Vec3::ZERO);
            let zone_entity = zone(&mut app, kind, 1.4, 0.15);
            let banner_entity = banner(&mut app, 1.4, 0.15);
            let base_speed = app.world().resource::<PlayerBuild>().attacks_per_second;
            let base_armor = app.world().resource::<HeroVitals>().armor;
            let mut elapsed = 0.0_f32;
            while elapsed < 3.2 {
                let step = delta.min(3.2 - elapsed);
                advance(&mut app, step);
                app.update();
                elapsed += step;
            }
            let actual = strike_count(&mut app);
            let repaired = app.world().resource::<GameSession>().core_hp - 50.0;
            println!(
                "ZONE_EXPIRY kind={kind:?} delta={delta} contacts={actual}/{expected} repaired={repaired}"
            );
            if actual != expected || (repaired - 2.0 * skill_utility_multiplier(21)).abs() >= 0.0001
            {
                discrepancies.push(format!(
                    "{kind:?} delta={delta} contacts={actual}/{expected} repaired={repaired}"
                ));
            }
            assert!(
                app.world_mut()
                    .query::<&SkillStrike>()
                    .iter(app.world())
                    .all(|strike| strike.target == target && strike.damage == 7.0)
            );

            assert!(app.world().get::<SkillZone>(zone_entity).is_none());
            assert!(app.world().get::<OathBanner>(banner_entity).is_none());
            assert!(app.world().get::<PlayerSummon>(banner_entity).is_none());
            assert_eq!(
                app.world().get::<Visibility>(banner_entity),
                Some(&Visibility::Hidden)
            );
            assert!(
                (app.world().resource::<PlayerBuild>().attacks_per_second - base_speed).abs()
                    < 0.0001
            );
            assert!((app.world().resource::<HeroVitals>().armor - base_armor).abs() < 0.0001);
        }
    }
    assert!(discrepancies.is_empty(), "{}", discrepancies.join("; "));
}

#[test]
fn paused_effects_preserve_clocks_then_emit_only_the_final_live_fragment() {
    let mut app = fixture();
    let target = enemy(&mut app, Vec3::X * 5.0);
    let zone_entity = zone(&mut app, ZoneKind::VortexColumn, 0.01, 0.005);
    let banner_entity = banner(&mut app, 0.01, 0.005);
    app.world_mut().resource_mut::<GameSession>().paused = true;
    advance(&mut app, 3.2);
    app.update();
    assert_eq!(strike_count(&mut app), 0);
    assert_eq!(app.world().resource::<GameSession>().core_hp, 50.0);
    assert_eq!(
        app.world().get::<SkillZone>(zone_entity).unwrap().remaining,
        0.01
    );
    assert_eq!(
        app.world()
            .get::<OathBanner>(banner_entity)
            .unwrap()
            .repair_remaining,
        0.005
    );
    assert_eq!(
        app.world().get::<Transform>(target).unwrap().translation.x,
        5.0
    );
    app.world_mut().resource_mut::<GameSession>().paused = false;
    advance(&mut app, 0.1);
    app.update();
    assert_eq!(strike_count(&mut app), 1);
    assert!(
        (app.world().resource::<GameSession>().core_hp - 50.0 - skill_utility_multiplier(21)).abs()
            < 0.0001
    );
    // Pull must use .01s of live time, not the entire .1s expired frame.
    assert!(
        (app.world().get::<Transform>(target).unwrap().translation.x - (5.0 - 3.8 * 0.01)).abs()
            < 0.0001
    );
    advance(&mut app, 3.2);
    app.update();
    assert_eq!(strike_count(&mut app), 1);
    assert!(
        (app.world().resource::<GameSession>().core_hp - 50.0 - skill_utility_multiplier(21)).abs()
            < 0.0001
    );
}

#[test]
fn already_expired_zone_cannot_attack_or_pull() {
    let mut app = fixture();
    let target = enemy(&mut app, Vec3::X * 5.0);
    let entity = zone(&mut app, ZoneKind::VortexColumn, 0.0, 0.15);
    advance(&mut app, 0.1);
    app.update();
    assert_eq!(strike_count(&mut app), 0);
    assert_eq!(
        app.world().get::<Transform>(target).unwrap().translation.x,
        5.0
    );
    assert!(app.world().get::<SkillZone>(entity).is_none());
}
