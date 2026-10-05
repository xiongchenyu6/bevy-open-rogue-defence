//! Real basic attacks drive automatic spells; no cast clocks or charge counters are forced.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, Autoplay, BasicAttackCounter, Enemy, EnemyArchetype, GameAssets, GameSession,
    Hero, HeroKind, HeroVitals, PlayerBuild, RecruitmentProgress, SelectedHero, SkillCastTelemetry,
    SpawnDirector,
};
#[derive(Resource, Default)]
struct Probe {
    frame: u32,
    last_total: u64,
    last_cast_time: f32,
    early_casts: u64,
    early_attacks: u64,
    late_start: u64,
    paused_attacks: u64,
    paused_casts: u64,
    saw_damage: bool,
}
#[derive(Component)]
struct Target;
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    *world.resource_mut::<SelectedHero>() = SelectedHero {
        kind: HeroKind::StormSavant,
        locked: true,
    };
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
    world.insert_resource(Probe::default());
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    for index in [0, 2, 3] {
        world.resource_mut::<PlayerBuild>().skills[index] = 1;
    }
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::StormSavant, 1);
    world.resource_mut::<GameSession>().level = 1;
    world
        .resource_mut::<RecruitmentProgress>()
        .claimed_milestones = 7;
    for mut transform in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        transform.translation = Vec3::ZERO;
    }
    let model = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for (index, z) in [-2.0, -1.0, 0.0, 1.0, 2.0].into_iter().enumerate() {
        world.spawn((
            Name::new(format!("Cadence real target {index}")),
            Target,
            WorldAssetRoot(model.clone()),
            EnemyArchetype(0),
            Transform::from_xyz(3.0 + index as f32 * 0.25, 0.0, z).with_scale(Vec3::splat(0.75)),
            Enemy {
                hp: 1_000_000.0,
                max_hp: 1_000_000.0,
                speed: 0.0,
                damage: 0.0,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    app.add_systems(PreUpdate, exercise)
        .add_systems(Last, observe);
}
fn exercise(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    if frame == 100 {
        let attacks = world.resource::<BasicAttackCounter>().count();
        let casts = world.resource::<SkillCastTelemetry>().total;
        let mut probe = world.resource_mut::<Probe>();
        probe.paused_attacks = attacks;
        probe.paused_casts = casts;
        world.resource_mut::<GameSession>().paused = true;
    }
    if frame == 111 {
        world.resource_mut::<GameSession>().paused = false;
    }
    if frame == 300 {
        let casts = world.resource::<SkillCastTelemetry>().total;
        let attacks = world.resource::<BasicAttackCounter>().count();
        let mut probe = world.resource_mut::<Probe>();
        probe.early_casts = casts;
        probe.early_attacks = attacks;
        probe.late_start = attacks;
        for index in [0, 2, 3] {
            world.resource_mut::<PlayerBuild>().skills[index] = 5;
        }
        world.resource_mut::<GameSession>().level = 40;
        println!("CADENCE_EARLY casts={casts} real_volleys={attacks}");
    }
}
fn observe(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    let total = world.resource::<SkillCastTelemetry>().total;
    let attacks = world.resource::<BasicAttackCounter>().count();
    let elapsed = world.resource::<GameSession>().elapsed;
    if total > world.resource::<Probe>().last_total {
        assert_eq!(
            total - world.resource::<Probe>().last_total,
            1,
            "damage spells must respect shared time gap"
        );
        let previous = world.resource::<Probe>().last_cast_time;
        if previous > 0.0 {
            assert!(
                elapsed - previous >= 0.16,
                "spells synchronized: dt={}",
                elapsed - previous
            );
        }
        let mut probe = world.resource_mut::<Probe>();
        probe.last_total = total;
        probe.last_cast_time = elapsed;
    }
    if frame <= 3 {
        assert_eq!(
            total, 0,
            "new skill cannot immediately cast before real attacks"
        );
    }
    if frame == 30 {
        assert!(
            world.resource::<ActiveSkill>().remaining > 0.0,
            "initial ultimate must wait hero cooldown"
        );
    }
    if (101..=110).contains(&frame) {
        assert_eq!(attacks, world.resource::<Probe>().paused_attacks);
        assert_eq!(total, world.resource::<Probe>().paused_casts);
    }
    let damage = world
        .query_filtered::<&Enemy, With<Target>>()
        .iter(world)
        .any(|enemy| enemy.hp < enemy.max_hp);
    world.resource_mut::<Probe>().saw_damage |= damage;
    if frame == 590 {
        let p = world.resource::<Probe>();
        let late = total - p.early_casts;
        assert!(
            p.early_attacks >= 8 && p.early_casts > 0,
            "early real attack charge did not cast"
        );
        assert!(
            late > p.early_casts,
            "rank5 should release faster than rank1: {} vs {late}",
            p.early_casts
        );
        assert!(
            attacks - p.late_start > p.early_attacks,
            "late natural basic attacks should accelerate"
        );
        assert!(p.saw_damage, "actual attacks must damage real targets");
        println!(
            "BALANCE_CADENCE_PASS early_casts={} late_casts={late} early_volleys={} late_volleys={} spacing>=0.16s initial_ultimate_wait=true pause_frozen=true damage=true",
            p.early_casts,
            p.early_attacks,
            attacks - p.late_start
        );
    }
}
