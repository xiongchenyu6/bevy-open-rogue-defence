//! Optional camps through their real offer, choice controls and combat resolver.
use bevy::prelude::*;
use bevy::{
    camera::RenderTarget,
    input::{ButtonState, mouse::MouseButtonInput},
    window::{CursorMoved, WindowRef},
};
use rogue_commander::game::{
    ADVENTURE_BOSSES, ADVENTURE_LANDING_SITES, ADVENTURE_OFFER_TIMES, ActiveSkill,
    AdventureBossChoice, AdventureBossState, AdventureDanger, AdventureImpactCue,
    AdventureSkillClock, AttackClock, Autoplay, BasicRelics, BossEnemy, CampArrival,
    EncounterGuard, Enemy, GameAssets, GameSession, Hero, HeroVitals, PlayerBuild,
    RecruitmentProgress, RelicCast, SelectedHero, SpawnDirector,
};

#[derive(Component)]
struct RelicTarget(usize);

#[derive(Resource, Default)]
struct Pointer {
    queued: Option<String>,
    saved_targets: Vec<(Entity, RenderTarget)>,
    down: bool,
}

#[derive(Clone, Copy)]
struct DangerProbe {
    kind: u8,
    target: Vec3,
    remaining_secs: f32,
    active: bool,
}
#[derive(Resource)]
struct Probe {
    scenario: String,
    index: usize,
    frame: u32,
    boss: Option<Entity>,
    main_boss: Option<Entity>,
    hero_origin: Vec3,
    arrival_seen: bool,
    landed: bool,
    warning: Option<u32>,
    frozen: Vec<(Entity, f32, bool)>,
    frozen_timer: f32,
    before_hp: f32,
    hit: bool,
    secondary: bool,
    phase: bool,
    impacts: Vec<(u8, u32)>,
    killing: bool,
    baseline: (u32, u32, u32),
    reward_at: Option<u32>,
    reset_at: Option<u32>,
    relic_started: bool,
    relic_cast: bool,
    relic_spectacle: bool,
    relic_damage: bool,
    relic_frozen: bool,
    relic_hp: Vec<(Entity, f32)>,
}

pub fn configure(world: &mut World, scenario: &str) {
    let index = scenario
        .strip_prefix("adventure-boss-")
        .map_or(0, |value| value.parse::<usize>().unwrap());
    assert!(index < 3);
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(Probe {
        scenario: scenario.into(),
        index,
        frame: 0,
        boss: None,
        main_boss: None,
        hero_origin: Vec3::ZERO,
        arrival_seen: false,
        landed: false,
        warning: None,
        frozen: vec![],
        frozen_timer: 0.,
        before_hp: 0.,
        hit: false,
        secondary: false,
        phase: false,
        impacts: Vec::new(),
        killing: false,
        baseline: (0, 0, 0),
        reward_at: None,
        reset_at: None,
        relic_started: false,
        relic_cast: false,
        relic_spectacle: false,
        relic_damage: false,
        relic_frozen: false,
        relic_hp: vec![],
    });
}

pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    let index = world.resource::<Probe>().index;
    world.resource_mut::<GameSession>().elapsed = ADVENTURE_OFFER_TIMES[index] + 1.;
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<ActiveSkill>().remaining = 10000.;
    world
        .resource_mut::<RecruitmentProgress>()
        .claimed_milestones = 7;
    {
        let mut hero = world.resource_mut::<HeroVitals>();
        hero.max_health = 100000.;
        hero.health = 100000.;
    }
    let origin = world
        .query_filtered::<&Transform, With<Hero>>()
        .single(world)
        .unwrap()
        .translation;
    world.resource_mut::<Probe>().hero_origin = origin;
    if world.resource::<Probe>().scenario == "adventure-main-boss" {
        let scene = world.resource::<GameAssets>().boss_scenes[0].clone();
        let main = world
            .spawn((
                Name::new("Concurrent main boss proof"),
                WorldAssetRoot(scene),
                Transform::from_xyz(14., 0., 5.),
                Visibility::Visible,
                BossEnemy { index: 0 },
                Enemy {
                    hp: 1000000.,
                    max_hp: 1000000.,
                    speed: 0.,
                    damage: 0.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: true,
                },
            ))
            .id();
        world.resource_mut::<Probe>().main_boss = Some(main);
    }
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
    app.add_systems(Last, (record_impacts, observe).chain());
    app.init_resource::<Pointer>()
        .add_systems(First, pointer_input);
}

fn stage(name: &str, frame: u32) {
    println!("[capture] adventure-stage {name} frame={frame}");
}
fn pointer_input(world: &mut World) {
    let (frame, index, small) = {
        let p = world.resource::<Probe>();
        (p.frame + 1, p.index, p.scenario == "adventure-small")
    };
    let label = match frame {
        15 => Some(format!("Adventure camp entry {index}")),
        50 => Some(
            if small {
                "Encounter boss safe reward"
            } else {
                "Encounter boss fight"
            }
            .into(),
        ),
        _ => world.resource_mut::<Pointer>().queued.take(),
    };
    let window = world
        .query::<Entity>()
        .iter(world)
        .find(|entity| world.get::<Window>(*entity).is_some())
        .expect("virtual capture window");
    if world.resource::<Pointer>().down {
        world.write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
        world.resource_mut::<Pointer>().down = false;
    }
    if let Some(label) = label {
        let position = world
            .query_filtered::<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>()
            .iter(world)
            .find(|(name, _, _)| name.as_str() == label)
            .map(|(_, node, t)| {
                assert!(
                    node.size().min_element() > 0.,
                    "actual button must have layout"
                );
                t.translation
            })
            .unwrap_or_else(|| panic!("missing actual adventure control {label}"));
        let scale = world.get::<Window>(window).unwrap().scale_factor();
        let logical = position / scale;
        world
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(logical));
        world.write_message(CursorMoved {
            window,
            position: logical,
            delta: None,
        });
        world.write_message(MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            window,
        });
        let targets: Vec<_> = world
            .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
            .iter(world)
            .map(|(entity, target)| (entity, target.clone()))
            .collect();
        for (entity, _) in &targets {
            *world.get_mut::<RenderTarget>(*entity).unwrap() =
                RenderTarget::Window(WindowRef::Entity(window));
        }
        world.resource_mut::<Pointer>().saved_targets = targets;
        world.resource_mut::<Pointer>().down = true;
        println!("[capture] actual virtual-window pointer {label} at {position:?} frame={frame}");
    }
}
fn place(world: &mut World, position: Vec3) {
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = position;
    }
}
fn wallet(world: &World) -> (u32, u32, u32) {
    let session = world.resource::<GameSession>();
    (
        session.gold,
        session.wood,
        world.resource::<RecruitmentProgress>().fruit_count,
    )
}
fn exercise(world: &mut World) {
    let targets = std::mem::take(&mut world.resource_mut::<Pointer>().saved_targets);
    for (entity, target) in targets {
        if let Some(mut current) = world.get_mut::<RenderTarget>(entity) {
            *current = target;
        }
    }
    world.resource_mut::<Probe>().frame += 1;
    let p = world.resource::<Probe>();
    let (frame, index, small) = (p.frame, p.index, p.scenario == "adventure-small");
    if frame == 10 {
        let scenes = world.resource::<GameAssets>().adventure_scenes.clone();
        for (index, scene) in scenes.iter().enumerate() {
            assert!(
                scenes
                    .iter()
                    .enumerate()
                    .all(|(other, handle)| other == index || handle != scene),
                "each camp has an independent model asset"
            );
            let assets = world.resource::<GameAssets>();
            assert!(
                assets
                    .enemy_scenes
                    .iter()
                    .chain(assets.boss_scenes.iter())
                    .all(|ordinary| ordinary != scene),
                "wild leader does not reuse an ordinary soldier or main boss"
            );
            let expected_name = format!("Resting side camp leader {index}");
            let preview = world
                .query::<(&Name, &WorldAssetRoot)>()
                .iter(world)
                .find(|(name, _)| name.as_str() == expected_name)
                .expect("all three real world camp leaders exist");
            assert_eq!(
                &preview.1.0, scene,
                "camp preview must use the independent live leader model"
            );
        }
        stage("three-independent-camp-models", frame);
        for index in 0..3 {
            assert!(
                world
                    .query::<(&Name, &Node)>()
                    .iter(world)
                    .any(
                        |(name, node)| name.as_str() == format!("Adventure camp entry {index}")
                            && node.display == Display::Flex
                    )
            );
        }
        stage("world-camp-entries", frame);
    }
    if frame == 35 {
        assert_eq!(
            world.resource::<AdventureBossState>().pending,
            Some(index as u8)
        );
        assert!(world.resource::<GameSession>().paused);
        if let Some(main) = world.resource::<Probe>().main_boss {
            assert!(
                world.get::<BossEnemy>(main).is_some() && world.get::<Enemy>(main).unwrap().hp > 0.
            );
            assert_eq!(world.resource::<AdventureBossState>().completed, 0);
            stage("choice-during-main-boss", frame);
        }
        stage("choice", frame);
    }
    if frame == 50 {
        world.resource_mut::<Probe>().baseline = wallet(world);
    }
    if frame == 75 && small {
        // A duplicate event from the same genuine input channel must not pay twice.
        world.write_message(AdventureBossChoice::TakeReward);
    }
    if frame == 65 && !small {
        let boss = world
            .resource::<AdventureBossState>()
            .active
            .expect("actual fight rents leader");
        let assets = world.resource::<GameAssets>();
        assert_eq!(
            world.get::<WorldAssetRoot>(boss).unwrap().0,
            assets.adventure_scenes[index]
        );
        assert!(
            assets
                .boss_scenes
                .iter()
                .all(|scene| *scene != world.get::<WorldAssetRoot>(boss).unwrap().0)
        );
        let guard_scenes: Vec<_> = world
            .query::<(&EncounterGuard, &WorldAssetRoot)>()
            .iter(world)
            .filter(|(guard, _)| guard.owner == boss)
            .map(|(_, scene)| scene.0.clone())
            .collect();
        assert_eq!(guard_scenes.len(), 3);
        for visual in [0, 3, 9] {
            assert!(guard_scenes.contains(&world.resource::<GameAssets>().enemy_scenes[visual]));
        }
        let arrivals: Vec<_> = world
            .query::<(Entity, &CampArrival, &Transform)>()
            .iter(world)
            .map(|(entity, arrival, transform)| {
                (entity, arrival.remaining_secs, transform.translation)
            })
            .collect();
        assert_eq!(arrivals.len(), 4);
        for (entity, remaining, position) in arrivals {
            assert!(remaining > 0. && position.y > 1., "real airborne jump arc");
            assert!(
                world.get::<Enemy>(entity).is_none() && world.get::<BossEnemy>(entity).is_none(),
                "airborne actors cannot enter real targeting queries"
            );
        }
        let origin = world.resource::<Probe>().hero_origin;
        assert_eq!(
            world
                .query_filtered::<&Transform, With<Hero>>()
                .single(world)
                .unwrap()
                .translation,
            origin,
            "fight does not teleport hero"
        );
        world.resource_mut::<Probe>().boss = Some(boss);
        world.resource_mut::<Probe>().arrival_seen = true;
        stage("arrival-airborne", frame);
    }
    if frame == 90 && !small {
        let boss = world.resource::<Probe>().boss.unwrap();
        assert!(world.get::<CampArrival>(boss).is_none());
        assert_eq!(
            world.get::<WorldAssetRoot>(boss).unwrap().0,
            world.resource::<GameAssets>().adventure_scenes[index],
            "landing retains the independent wild leader model"
        );
        assert!(world.get::<Enemy>(boss).is_some() && world.get::<BossEnemy>(boss).is_none());
        assert!(world.get::<AdventureSkillClock>(boss).is_some());
        assert_eq!(
            world
                .query::<(&EncounterGuard, &Enemy)>()
                .iter(world)
                .filter(|(guard, enemy)| guard.owner == boss && enemy.hp > 0.)
                .count(),
            3
        );
        assert_eq!(world.resource::<AdventureBossState>().completed, 0);
        assert!(
            !world.resource::<BasicRelics>().owns(index as u8),
            "landing or concurrent main boss cannot grant camp victory"
        );
        assert_eq!(
            world.resource::<RecruitmentProgress>().fruit_count,
            world.resource::<Probe>().baseline.2
        );
        let landing = ADVENTURE_LANDING_SITES[index];
        let position = world.get::<Transform>(boss).unwrap().translation;
        assert!(position.y.abs() < 0.01 && (position.z - landing.z).abs() < 1.);
        world.get_mut::<Enemy>(boss).unwrap().speed = 0.;
        let guards: Vec<_> = world
            .query::<(Entity, &EncounterGuard)>()
            .iter(world)
            .filter(|(_, guard)| guard.owner == boss)
            .map(|(entity, _)| entity)
            .collect();
        for guard in guards {
            let mut enemy = world.get_mut::<Enemy>(guard).unwrap();
            enemy.damage = 0.;
            enemy.speed = 0.;
        }
        place(world, landing - Vec3::X * 3.);
        world.resource_mut::<Probe>().landed = true;
        let mut session = world.resource_mut::<GameSession>();
        session.core_hp = 0.;
        session.gate_revive_remaining = 10000.;
        stage("arrival-landed", frame);
        stage("camp-battle", frame);
    }
    if world.resource::<Probe>().killing {
        let boss = world.resource::<Probe>().boss.unwrap();
        let targets: Vec<_> = world
            .query::<(Entity, &Enemy, &Transform, Option<&EncounterGuard>)>()
            .iter(world)
            .filter(|(entity, enemy, _, guard)| {
                enemy.hp > 0. && (*entity == boss || guard.is_some_and(|guard| guard.owner == boss))
            })
            .map(|(entity, _, transform, _)| (entity, transform.translation))
            .collect();
        if let Some((target, position)) = targets.first() {
            world.get_mut::<Enemy>(*target).unwrap().hp = 1.;
            place(world, *position - Vec3::X * 1.2);
            let mut clock = world.resource_mut::<AttackClock>();
            clock.0.unpause();
            let duration = clock.0.duration();
            clock.0.set_elapsed(duration);
        }
    }
    if world.resource::<Probe>().relic_started && !world.resource::<Probe>().relic_cast {
        let mut clock = world.resource_mut::<AttackClock>();
        clock.0.unpause();
        let duration = clock.0.duration();
        clock.0.set_elapsed(duration);
    }
}

fn start_relic_targets(world: &mut World) {
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    place(world, Vec3::ZERO);
    for (index, position) in [
        Vec3::new(3., 0., 0.),
        Vec3::new(4., 0., 2.),
        Vec3::new(4., 0., -2.),
        Vec3::new(4.5, 0., 0.),
    ]
    .into_iter()
    .enumerate()
    {
        world.spawn((
            Name::new(format!("Adventure relic proof target {index}")),
            RelicTarget(index),
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(0.8)),
            Enemy {
                hp: 100000.,
                max_hp: 100000.,
                speed: 0.4,
                damage: 0.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    world.resource_mut::<GameSession>().paused = false;
    world.resource_mut::<Probe>().relic_started = true;
}

fn record_impacts(mut messages: MessageReader<AdventureImpactCue>, mut probe: ResMut<Probe>) {
    for impact in messages.read() {
        if Some(impact.owner) == probe.boss && usize::from(impact.index) == probe.index {
            let frame = probe.frame;
            probe.impacts.push((impact.kind, frame));
            println!(
                "[capture] native adventure impact kind={} frame={frame}",
                impact.kind
            );
        }
    }
}
fn observe(world: &mut World) {
    let p = world.resource::<Probe>();
    let (frame, index, small, loss, boss) = (
        p.frame,
        p.index,
        p.scenario == "adventure-small",
        p.scenario == "adventure-loss",
        p.boss,
    );
    if small {
        if frame == 51 {
            let before = world.resource::<Probe>().baseline;
            let after = wallet(world);
            let def = ADVENTURE_BOSSES[index];
            assert!(
                after.0 >= before.0 + def.small_gold,
                "safe reward includes its promised gold; passive income may also accrue"
            );
            assert_eq!(after.1, before.1 + def.small_wood);
            assert_eq!(after.2, before.2);
            assert!(world.resource::<AdventureBossState>().active.is_none());
            assert_eq!(world.resource::<AdventureBossState>().completed, 0);
            assert!((0..3).all(|index| !world.resource::<BasicRelics>().owns(index as u8)));
            world.resource_mut::<GameSession>().paused = true;
            world.resource_mut::<Probe>().baseline = after;
            stage("safe-reward-no-battle", frame);
        }
        if frame == 90 {
            assert_eq!(wallet(world), world.resource::<Probe>().baseline);
            assert!(
                world
                    .query::<&EncounterGuard>()
                    .iter(world)
                    .next()
                    .is_none()
            );
            stage("safe-reward-once", frame);
        }
    } else if let Some(boss) = boss {
        let hazards: Vec<_> = world
            .query::<(Entity, &AdventureDanger)>()
            .iter(world)
            .filter(|(_, hazard)| hazard.owner == boss)
            .map(|(entity, hazard)| {
                (
                    entity,
                    DangerProbe {
                        kind: hazard.kind,
                        target: hazard.target,
                        remaining_secs: hazard.remaining_secs,
                        active: hazard.active,
                    },
                )
            })
            .collect();
        if world.resource::<Probe>().warning.is_none()
            && let Some((_, hazard)) = hazards.iter().find(|(_, hazard)| !hazard.active)
        {
            assert_eq!(
                world.get::<WorldAssetRoot>(boss).unwrap().0,
                world.resource::<GameAssets>().adventure_scenes[index],
                "real combat never falls back to an ordinary soldier model"
            );
            world.resource_mut::<Probe>().warning = Some(frame);
            world.resource_mut::<Probe>().before_hp = world.resource::<HeroVitals>().health;
            place(world, hazard.target);
            stage("primary-warning", frame);
        }
        if let Some(warning) = world.resource::<Probe>().warning {
            if frame == warning + 5 {
                world.resource_mut::<GameSession>().paused = true;
                let frozen = hazards
                    .iter()
                    .map(|(entity, h)| (*entity, h.remaining_secs, h.active))
                    .collect();
                world.resource_mut::<Probe>().frozen = frozen;
                world.resource_mut::<Probe>().frozen_timer =
                    world.resource::<AdventureBossState>().remaining_secs;
                stage("warning-paused", frame);
            }
            if frame == warning + 20 {
                let current: Vec<_> = hazards
                    .iter()
                    .map(|(entity, h)| (*entity, h.remaining_secs, h.active))
                    .collect();
                assert_eq!(current, world.resource::<Probe>().frozen);
                assert_eq!(
                    world.resource::<AdventureBossState>().remaining_secs,
                    world.resource::<Probe>().frozen_timer
                );
                world.resource_mut::<GameSession>().paused = false;
                if loss {
                    let mut hero = world.resource_mut::<HeroVitals>();
                    hero.deaths = 2;
                    hero.health = 1.;
                }
            }
        }
        if !loss
            && !world.resource::<Probe>().hit
            && world.resource::<HeroVitals>().health < world.resource::<Probe>().before_hp
            && (hazards.iter().any(|(_, h)| h.active)
                || !world.resource::<Probe>().impacts.is_empty())
        {
            world.resource_mut::<Probe>().hit = true;
            let mut enemy = world.get_mut::<Enemy>(boss).unwrap();
            enemy.hp = enemy.max_hp * 0.49;
            stage("actual-hazard-hit", frame);
        }
        if !loss
            && !world.resource::<Probe>().phase
            && world.resource::<Probe>().hit
            && world
                .get::<AdventureSkillClock>(boss)
                .is_some_and(|clock| clock.enraged)
        {
            world.resource_mut::<Probe>().phase = true;
            stage("phase-two", frame);
        }
        if !loss
            && world.resource::<Probe>().phase
            && !world.resource::<Probe>().secondary
            && (hazards.iter().any(|(_, h)| h.kind == 1 && h.active)
                || world
                    .resource::<Probe>()
                    .impacts
                    .iter()
                    .any(|(kind, _)| *kind == 1))
        {
            world.resource_mut::<Probe>().secondary = true;
            stage("secondary-active", frame);
        }
        if !loss
            && world.resource::<Probe>().secondary
            && !world.resource::<Probe>().killing
            && frame % 10 == 0
        {
            world.resource_mut::<Probe>().baseline = wallet(world);
            world.resource_mut::<Probe>().killing = true;
            stage("native-attacks-finish-camp", frame);
        }
        if !loss
            && world.resource::<AdventureBossState>().completed & (1 << index) != 0
            && world.resource::<Probe>().reward_at.is_none()
        {
            let before = world.resource::<Probe>().baseline;
            let after = wallet(world);
            let def = ADVENTURE_BOSSES[index];
            assert!(after.0 >= before.0 + def.reward_gold);
            assert_eq!(after.1, before.1 + def.reward_wood);
            assert_eq!(after.2, before.2 + 1);
            world.resource_mut::<Probe>().reward_at = Some(frame);
            world.resource_mut::<Probe>().baseline = after;
            world.resource_mut::<GameSession>().paused = true;
            stage("victory-reward", frame);
        }
        if loss
            && world.resource::<GameSession>().finished
            && world.resource::<Probe>().reward_at.is_none()
        {
            assert!(!world.resource::<GameSession>().victory);
            assert_eq!(world.resource::<AdventureBossState>().completed, 0);
            world.resource_mut::<Probe>().reward_at = Some(frame);
            stage("last-life-failure", frame);
        }
        if let Some(done) = world.resource::<Probe>().reward_at
            && frame == done + 15
        {
            assert!(
                hazards.is_empty(),
                "real victory/failure cleans hazards while paused"
            );
            assert!(
                world
                    .query::<&EncounterGuard>()
                    .iter(world)
                    .all(|guard| guard.owner != boss)
            );
            assert!(world.resource::<AdventureBossState>().active.is_none());
            if !loss {
                assert_eq!(wallet(world), world.resource::<Probe>().baseline);
                assert!(
                    world.resource::<BasicRelics>().owns(index as u8),
                    "genuine camp victory grants its unique relic"
                );
            }
            stage("paused-cleanup-reward-once", frame);
            if loss {
                world.resource_mut::<Pointer>().queued = Some("Campaign retry".into());
                world.resource_mut::<Probe>().reset_at = Some(frame);
            }
        }
    }
    if !loss
        && !small
        && world
            .resource::<Probe>()
            .reward_at
            .is_some_and(|at| frame == at + 30)
    {
        start_relic_targets(world);
        stage("awarded-relic-basic-attacks", frame);
    }
    if world.resource::<Probe>().relic_started {
        if !world.resource::<Probe>().relic_cast
            && world
                .query::<&RelicCast>()
                .iter(world)
                .any(|cast| usize::from(cast.index) == index)
        {
            assert!(world.resource::<BasicRelics>().casts_emitted[index] > 0);
            let hp = world
                .query::<(Entity, &RelicTarget, &Enemy)>()
                .iter(world)
                .filter(|(_, target, _)| matches!(target.0, 1 | 2))
                .map(|(entity, _, enemy)| (entity, enemy.hp))
                .collect();
            world.resource_mut::<Probe>().relic_hp = hp;
            world.resource_mut::<Probe>().relic_cast = true;
            world.resource_mut::<AttackClock>().0.pause();
            stage("real-relic-cast", frame);
        }
        if !world.resource::<Probe>().relic_spectacle
            && world
                .query::<&Name>()
                .iter(world)
                .any(|name| name.as_str() == format!("Relic {index} spectacle"))
        {
            world.resource_mut::<Probe>().relic_spectacle = true;
            stage("relic-spectacle", frame);
        }
        let damaged = world
            .resource::<Probe>()
            .relic_hp
            .iter()
            .any(|(entity, hp)| {
                world
                    .get::<Enemy>(*entity)
                    .is_some_and(|enemy| enemy.hp < *hp)
            });
        if world.resource::<Probe>().relic_spectacle
            && damaged
            && !world.resource::<Probe>().relic_damage
        {
            world.resource_mut::<Probe>().relic_damage = true;
            stage("relic-area-damage", frame);
        }
        if index == 1
            && world.resource::<Probe>().relic_spectacle
            && world
                .query::<(&RelicTarget, &Enemy)>()
                .iter(world)
                .any(|(_, enemy)| enemy.speed == 0.)
        {
            if !world.resource::<Probe>().relic_frozen {
                stage("relic-actual-freeze", frame);
            }
            world.resource_mut::<Probe>().relic_frozen = true;
        }
    }
    if let Some(reset) = world.resource::<Probe>().reset_at
        && frame == reset + 20
    {
        let state = world.resource::<AdventureBossState>();
        assert!(state.active.is_none() && state.pending.is_none());
        assert_eq!(state.completed, 0);
        assert!(!world.resource::<GameSession>().finished);
        stage("real-retry-reset", frame);
    }
    if frame == 990 {
        let p = world.resource::<Probe>();
        assert!(small || (p.arrival_seen && p.landed));
        if let Some(main) = p.main_boss {
            assert!(
                world.get::<Enemy>(main).is_some_and(|enemy| enemy.hp > 0.),
                "main boss remains independent from camp victory"
            );
        }
        assert!(
            small
                || (loss && p.reset_at.is_some())
                || (!loss
                    && p.hit
                    && p.phase
                    && p.secondary
                    && p.reward_at.is_some()
                    && p.relic_cast
                    && p.relic_spectacle
                    && p.relic_damage
                    && (index != 1 || p.relic_frozen)),
            "all actual adventure stages reached"
        );
        println!(
            "[capture] PASS {}: real choice/combat/reward/cleanup across 1000 frames",
            p.scenario
        );
    }
}
