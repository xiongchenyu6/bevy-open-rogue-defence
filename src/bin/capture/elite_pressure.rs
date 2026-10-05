//! Create each elite through the actual debug input menu, then observe native traits.
use bevy::{prelude::*, ui::UiSystems};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, EliteAffix, EliteAffixKind, ElitePressure, Enemy,
    EnemyArchetype, GameSession, Hero, HeroVitals, PlayerBuild, SelectedHero, SpawnDirector,
};
#[derive(Resource)]
struct EliteProbe {
    frame: u32,
    index: usize,
    owner: Option<Entity>,
    shield: bool,
    pending: bool,
    wounded: f32,
    regenerated: bool,
    yellow_entered: bool,
    yellow_left: bool,
}
pub fn configure(world: &mut World, scenario: &str) {
    let index = scenario
        .strip_prefix("elite-pressure-")
        .unwrap()
        .parse::<usize>()
        .unwrap();
    assert!(index < 8);
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(EliteProbe {
        frame: 0,
        index,
        owner: None,
        shield: false,
        pending: false,
        wounded: 0.,
        regenerated: false,
        yellow_entered: false,
        yellow_left: false,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    let mut session = world.resource_mut::<GameSession>();
    session.core_max_hp = 100000.;
    session.core_hp = 100000.;
    let mut vitals = world.resource_mut::<HeroVitals>();
    vitals.max_health = 100000.;
    vitals.health = 100000.;
    app.add_systems(
        PreUpdate,
        observe
            .after(UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn observe(world: &mut World) {
    world.resource_mut::<EliteProbe>().frame += 1;
    let state = world.resource::<EliteProbe>();
    let (frame, index) = (state.frame, state.index);
    world.resource_mut::<ButtonInput<KeyCode>>().clear();
    for key in [KeyCode::F7, KeyCode::Tab, KeyCode::Equal, KeyCode::Enter] {
        world.resource_mut::<ButtonInput<KeyCode>>().release(key);
    }
    let key = match frame {
        10 => Some(KeyCode::F7),
        15 => Some(KeyCode::Tab),
        50 => Some(KeyCode::Enter),
        60 => Some(KeyCode::F7),
        n if (20..20 + index as u32 * 3).contains(&n) && (n - 20).is_multiple_of(3) => {
            Some(KeyCode::Equal)
        }
        _ => None,
    };
    if let Some(key) = key {
        world.resource_mut::<ButtonInput<KeyCode>>().press(key);
    }
    if frame == 70 {
        let expected = [
            EliteAffixKind::Frenzy,
            EliteAffixKind::Bulwark,
            EliteAffixKind::Volatile,
            EliteAffixKind::Regeneration,
            EliteAffixKind::Commander,
            EliteAffixKind::YellowSign,
            EliteAffixKind::Brood,
            EliteAffixKind::Siege,
        ][index];
        let owners: Vec<_> = world
            .query::<(Entity, &EliteAffix)>()
            .iter(world)
            .filter(|(_, affix)| affix.kind() == expected)
            .map(|(entity, _)| entity)
            .collect();
        assert_eq!(
            owners.len(),
            1,
            "actual debug handler spawns selected real trait"
        );
        let owner = owners[0];
        world.resource_mut::<EliteProbe>().owner = Some(owner);
        world.get_mut::<Transform>(owner).unwrap().translation = Vec3::new(7., 0., 0.);
        assert!(
            world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == "Elite affix crest")
        );
        println!("[capture] elite-stage identity frame={frame} actual={expected:?}");
    }
    if let Some(owner) = world.resource::<EliteProbe>().owner {
        if world
            .get::<EliteAffix>(owner)
            .is_some_and(|affix| affix.shield_active())
            && !world.resource::<EliteProbe>().shield
        {
            world.resource_mut::<EliteProbe>().shield = true;
            println!("[capture] elite-stage shield frame={frame}");
        }
        if world
            .get::<EliteAffix>(owner)
            .is_some_and(|affix| affix.is_volatile_pending())
            && !world.resource::<EliteProbe>().pending
        {
            world.resource_mut::<EliteProbe>().pending = true;
            println!("[capture] elite-stage explosion-warning frame={frame}");
        }
        if index == 3 && frame == 100 {
            let mut enemy = world.get_mut::<Enemy>(owner).unwrap();
            enemy.hp = enemy.max_hp * 0.6;
            let hp = enemy.hp;
            world.resource_mut::<EliteProbe>().wounded = hp;
        }
        if index == 3 && frame == 200 {
            let hp = world.get::<Enemy>(owner).unwrap().hp;
            assert!(
                hp > world.resource::<EliteProbe>().wounded,
                "real bloodrite self-heals wounded health"
            );
            world.resource_mut::<EliteProbe>().regenerated = true;
            println!("[capture] elite-stage regeneration frame={frame}");
        }
        if index == 5 && frame == 100 {
            let position = world.get::<Transform>(owner).unwrap().translation + Vec3::Z * 2.;
            for mut transform in world
                .query_filtered::<&mut Transform, With<Hero>>()
                .iter_mut(world)
            {
                transform.translation = position;
            }
        }
        if index == 5 && frame == 115 {
            assert!(world.resource::<ElitePressure>().yellow_sign_active);
            assert_eq!(
                world
                    .resource::<ElitePressure>()
                    .cooldown_recovery_multiplier(),
                0.5
            );
            world.resource_mut::<EliteProbe>().yellow_entered = true;
            println!("[capture] elite-stage yellow-aura frame={frame}");
        }
        if index == 5 && frame == 130 {
            for mut transform in world
                .query_filtered::<&mut Transform, With<Hero>>()
                .iter_mut(world)
            {
                transform.translation = Vec3::new(-10., 0., 0.);
            }
        }
        if index == 5 && frame == 145 {
            assert!(!world.resource::<ElitePressure>().yellow_sign_active);
            assert_eq!(
                world
                    .resource::<ElitePressure>()
                    .cooldown_recovery_multiplier(),
                1.
            );
            world.resource_mut::<EliteProbe>().yellow_left = true;
        }
        if (index == 2 || index == 6) && frame == 550 {
            if index == 6 {
                let mut director = world.resource_mut::<SpawnDirector>();
                director.enabled = true;
                director.boss_spawned.fill(true);
                director.next_elite_at = 10000.;
            }
            world.get_mut::<Enemy>(owner).unwrap().hp = 1.;
            let position = world.get::<Transform>(owner).unwrap().translation - Vec3::X * 2.;
            for mut transform in world
                .query_filtered::<&mut Transform, With<Hero>>()
                .iter_mut(world)
            {
                transform.translation = position;
            }
            let mut clock = world.resource_mut::<AttackClock>();
            clock.0.unpause();
            let duration = clock.0.duration();
            clock.0.set_elapsed(duration);
        }
        if (index == 2 || index == 6)
            && frame > 550
            && (world.get::<Enemy>(owner).is_none() || world.resource::<EliteProbe>().pending)
        {
            world.resource_mut::<AttackClock>().0.pause();
        }
        if index == 6 && frame == 620 {
            assert!(
                world
                    .query::<(&EnemyArchetype, Option<&EliteAffix>, &Enemy, &Name)>()
                    .iter(world)
                    .filter(|(kind, affix, enemy, name)| kind.0 == 1
                        && affix.is_none()
                        && enemy.hp > 0.
                        && name.as_str().starts_with("SPLITLING"))
                    .count()
                    >= 3,
                "real brood death releases non-recursive catalog children"
            );
            println!("[capture] elite-stage brood-split frame={frame}");
        }
    }
    if frame == 640 {
        let state = world.resource::<EliteProbe>();
        assert!(state.owner.is_some());
        match index {
            1 => assert!(state.shield),
            2 => assert!(state.pending),
            3 => assert!(state.regenerated),
            5 => assert!(state.yellow_entered && state.yellow_left),
            _ => {}
        }
        println!(
            "[capture] PASS elite-pressure-{index}: real debug purchase-free spawn, unique active affix component/crest, native trait observation"
        );
    }
}
