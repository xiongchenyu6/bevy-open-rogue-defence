//! Authoritative skill casts, pursuit, retargeting and pause; no hand-spawned summons.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals,
    PlayerBuild, PlayerSummon, SelectedHero, SpawnDirector,
};
use std::collections::HashMap;
const FIRST: Vec3 = Vec3::new(4.0, 0.0, 0.0);
const SECOND: Vec3 = Vec3::new(-1.0, 0.0, -5.0);
const LABELS: [&str; 6] = [
    "skeleton",
    "hound",
    "water elemental",
    "frost wyrm",
    "colossus",
    "spirit companion",
];
const RANGES: [f32; 6] = [1.1, 1.1, 6.0, 7.0, 2.6, 5.35];
#[derive(Component)]
struct AssaultTarget;
#[derive(Resource)]
struct AssaultCapture {
    frame: u32,
    first: [Option<f32>; 6],
    closest: [f32; 6],
    retarget_start: [Option<f32>; 6],
    retarget_best: [f32; 6],
    walk: [bool; 5],
    attack: [bool; 5],
    damage: [bool; 2],
    frozen: HashMap<Entity, Transform>,
}
impl Default for AssaultCapture {
    fn default() -> Self {
        Self {
            frame: 0,
            first: [None; 6],
            closest: [f32::MAX; 6],
            retarget_start: [None; 6],
            retarget_best: [f32::MAX; 6],
            walk: [false; 5],
            attack: [false; 5],
            damage: [false; 2],
            frozen: HashMap::new(),
        }
    }
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SelectedHero>().kind = HeroKind::SpiritKeeper;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::SpiritKeeper, 1);
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = true;
}
fn target(commands: &mut Commands, scene: Handle<WorldAsset>, position: Vec3) {
    commands.spawn((
        Name::new("Summon assault target"),
        AssaultTarget,
        WorldAssetRoot(scene),
        Transform::from_translation(position).with_scale(Vec3::splat(0.9)),
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
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    for index in [4, 5, 16, 17, 26, 27] {
        world.resource_mut::<PlayerBuild>().skills[index] = 1;
    }
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.pause();
    let center = Vec3::new(-6.0, 0.0, 0.0);
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = center;
    }
    for (mut camera, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *camera = Transform::from_xyz(8., 24., 21.).looking_at(Vec3::ZERO, Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 22.0,
            };
        }
    }
    app.init_resource::<AssaultCapture>()
        .add_systems(PreUpdate, exercise);
}
fn kind(model: Option<&WorldAssetRoot>, name: Option<&Name>, assets: &GameAssets) -> Option<usize> {
    if name.is_some_and(|name| name.as_str() == "Spirit companion") {
        return Some(5);
    }
    let model = model?;
    [0, 1, 2, 4, 5]
        .into_iter()
        .position(|index| model.0 == assets.summon_scenes[index])
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn exercise(
    mut commands: Commands,
    mut state: ResMut<AssaultCapture>,
    mut session: ResMut<GameSession>,
    assets: Res<GameAssets>,
    models: Query<
        (
            Entity,
            Option<&WorldAssetRoot>,
            Option<&Name>,
            &Transform,
            &Visibility,
        ),
        Without<Enemy>,
    >,
    mut targets: Query<(&mut Enemy, &Transform), With<AssaultTarget>>,
    players: Query<(Entity, &AnimationPlayer)>,
    parents: Query<&ChildOf>,
    active_summons: Query<(), With<PlayerSummon>>,
) {
    state.frame += 1;
    let frame = state.frame;
    if frame == 1 {
        target(&mut commands, assets.enemy_scenes[0].clone(), FIRST);
    }
    let phase = usize::from(frame >= 180);
    state.damage[phase] |= targets
        .iter()
        .any(|(enemy, _)| enemy.hp > 0.0 && enemy.hp < enemy.max_hp);
    if frame == 180 {
        for (mut enemy, _) in &mut targets {
            enemy.hp = 0.0;
        }
        target(&mut commands, assets.enemy_scenes[0].clone(), SECOND);
        // Exclude damage to the retired first target from the new phase.
        state.damage[1] = false;
    }
    let destination = targets
        .iter()
        .find_map(|(enemy, transform)| (enemy.hp > 0.0).then_some(transform.translation))
        .unwrap_or(if phase == 0 { FIRST } else { SECOND });
    for (entity, model, name, transform, visibility) in &models {
        if *visibility == Visibility::Hidden {
            continue;
        }
        let Some(index) = kind(model, name, &assets) else {
            continue;
        };
        let distance = (transform.translation - destination).with_y(0.0).length();
        if phase == 0 {
            state.first[index].get_or_insert(distance);
            state.closest[index] = state.closest[index].min(distance);
        } else {
            state.retarget_start[index].get_or_insert(distance);
            state.retarget_best[index] = state.retarget_best[index].min(distance);
        }
        if frame == 250 {
            state.frozen.insert(entity, *transform);
        }
    }
    for (entity, player) in &players {
        let mut root = entity;
        for _ in 0..24 {
            if let Ok((_, model, name, _, visibility)) = models.get(root)
                && *visibility != Visibility::Hidden
                && let Some(index) = kind(model, name, &assets)
                && index < 5
            {
                // AnimationGraph::from_clips reserves node 0 for the root; clips Idle/Walk/Attack are nodes 1/2/3.
                state.walk[index] |= player.animation(AnimationNodeIndex::new(2)).is_some();
                state.attack[index] |= player.animation(AnimationNodeIndex::new(3)).is_some();
                break;
            }
            let Ok(parent) = parents.get(root) else {
                break;
            };
            root = parent.parent();
        }
    }
    if frame == 170 {
        for (index, label) in LABELS.iter().enumerate() {
            let initial =
                state.first[index].expect("all six summon families must spawn by real casting");
            assert!(
                initial - state.closest[index] > 0.35,
                "{} did not pursue: {initial} -> {}",
                label,
                state.closest[index]
            );
            assert!(
                state.closest[index] <= RANGES[index] + 0.45,
                "{} never reached attack range",
                label
            );
        }
        assert!(state.damage[0]);
        println!(
            "[capture] PASS: all six summon types pursue and reach range; actual enemy HP decreased; closest={:?}",
            state.closest
        );
    }
    if frame == 470 {
        for (mut enemy, _) in &mut targets {
            enemy.hp = 0.;
        }
    }
    if frame == 510 || frame == 650 {
        let mut families = [false; 6];
        for (entity, model, name, transform, visibility) in &models {
            if *visibility == Visibility::Hidden || !active_summons.contains(entity) {
                continue;
            }
            if let Some(index) = kind(model, name, &assets) {
                assert!(
                    transform.translation.x >= -5.2 + 1.8 - 0.001,
                    "idle summons must remain on enemy side of wall: {:?}",
                    transform.translation
                );
                let forward = transform.rotation * Vec3::NEG_Z;
                assert!(
                    forward.x > 0.9,
                    "idle active summon must face enemy entry: family={}, entity={entity:?}, name={name:?}, forward={forward:?}, live_targets={}",
                    LABELS[index],
                    targets.iter().filter(|(enemy, _)| enemy.hp > 0.).count()
                );
                families[index] = true;
            }
            if model.is_some_and(|model| model.0 == assets.summon_scenes[3]) {
                assert!(
                    (transform.translation - Vec3::new(-5.2, 3.1, 0.)).length() < 0.01,
                    "bone turret anchors above wall: {:?}",
                    transform.translation
                );
            }
        }
        if frame == 510 {
            assert!(
                families.iter().all(|present| *present),
                "all six mobile summon families idle outside wall: {families:?}"
            );
        }
        println!(
            "[capture] PASS outside summon formation frame={frame}: active families={families:?}, east of wall, face entry; bone turret above wall"
        );
    }
    if frame == 520 {
        session.core_hp = 0.;
    }
    if frame == 540 {
        assert!(session.gate_revive_remaining > 0.);
    }
    if frame == 650 {
        assert_eq!(session.core_hp, session.core_max_hp);
        assert_eq!(session.gate_revive_remaining, 0.);
        println!(
            "[capture] PASS summon wall rebuild: idle frontline persists, finite-health wall restored"
        );
    }
    if frame == 250 {
        session.paused = true;
    }
    if frame == 279 {
        assert!(!state.frozen.is_empty());
        for (entity, before) in &state.frozen {
            let (_, _, _, after, _) = models.get(*entity).expect("paused summons remain alive");
            assert_eq!(before.translation, after.translation);
            assert_eq!(before.rotation, after.rotation);
        }
        println!(
            "[capture] PASS: {} active summon transforms frozen for 29 frames",
            state.frozen.len()
        );
    }
    if frame == 280 {
        session.paused = false;
    }
    if frame == 390 {
        assert!(state.damage[1], "summons must damage replacement target");
        for (index, label) in LABELS.iter().enumerate() {
            let initial = state.retarget_start[index].expect("each summon family retargets");
            assert!(
                state.retarget_best[index] <= RANGES[index] + 0.45,
                "{} did not close to replacement enemy",
                label
            );
            assert!(
                initial <= RANGES[index] + 0.45 || initial - state.retarget_best[index] > 0.35,
                "{} failed to re-engage",
                label
            );
        }
        println!(
            "[capture] PASS: dead enemy replaced, all summons re-engage live target; walk={:?}, attack={:?}",
            state.walk, state.attack
        );
        assert!(
            state.walk.iter().all(|seen| *seen),
            "every scene summon should visibly walk"
        );
        assert!(
            state.attack.iter().all(|seen| *seen),
            "every scene summon should visibly attack"
        );
    }
}
