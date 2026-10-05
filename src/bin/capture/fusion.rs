//! Two actual fruit recruitments; retained builds and real legacy attacks.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, Autoplay, Enemy, GameAssets, GameSession, Hero, HeroFusion, HeroKind, PlayerBuild,
    RecruitmentProgress, SelectedHero, SkillId, SpawnDirector, content::HEROES,
    skill_evolution::SkillEvolution,
};
#[derive(Resource, Default)]
struct FusionCapture {
    frame: u32,
    before: [u8; SkillId::COUNT],
    tiers: [u8; SkillId::COUNT],
    first: Option<HeroKind>,
    second: Option<HeroKind>,
    candidate: Option<HeroKind>,
    damage: bool,
}
#[derive(Component)]
struct FusionTarget;
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SelectedHero>().kind = HeroKind::StormSavant;
    world
        .resource_mut::<HeroFusion>()
        .register_starting_skill(HeroKind::StormSavant, SkillId(0));
    world.resource_mut::<SpawnDirector>().enabled = false;
    {
        let mut session = world.resource_mut::<GameSession>();
        session.level = 40;
        session.elapsed = 60.0;
        session.skill_points = 0;
        session.auto_bond_draws = false;
    }
    {
        let mut recruitment = world.resource_mut::<RecruitmentProgress>();
        recruitment.claimed_milestones = 7;
        recruitment.fruit_count = 2;
    }
    let mut ranks = [0; SkillId::COUNT];
    for index in [0, 1, 4, 6, 16, 30] {
        ranks[index] = 5;
    }
    world.resource_mut::<PlayerBuild>().skills = ranks;
    world.resource_mut::<SkillEvolution>().tiers[6] = 1;
    let tiers = world.resource::<SkillEvolution>().tiers;
    world.insert_resource(FusionCapture {
        before: ranks,
        tiers,
        ..default()
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    let center = Vec3::new(-4.0, 0.0, 0.0);
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = center;
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for z in [-0.6, 0.0, 0.6] {
        world.spawn((
            Name::new("Fusion durable target"),
            FusionTarget,
            WorldAssetRoot(scene.clone()),
            Transform::from_xyz(-2.7, 0.0, z).with_scale(Vec3::splat(0.65)),
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
    for (mut camera, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *camera =
            Transform::from_xyz(-3.0, 12.0, 10.0).looking_at(Vec3::new(-3.0, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 11.0,
            };
        }
    }
    app.add_systems(
        PreUpdate,
        exercise
            .after(bevy::ui::UiSystems::Focus)
            .before(rogue_commander::game::SettingsInputSet),
    );
}
fn portrait_kind(
    entity: Entity,
    children: &Query<&Children>,
    images: &Query<&ImageNode>,
    handles: &[(HeroKind, Handle<Image>)],
) -> Option<HeroKind> {
    if let Ok(image) = images.get(entity)
        && let Some((kind, _)) = handles.iter().find(|(_, handle)| *handle == image.image)
    {
        return Some(*kind);
    }
    children
        .get(entity)
        .ok()?
        .iter()
        .find_map(|child| portrait_kind(child, children, images, handles))
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn exercise(
    mut state: ResMut<FusionCapture>,
    mut recruitment: ResMut<RecruitmentProgress>,
    selected: Res<SelectedHero>,
    build: Res<PlayerBuild>,
    evolution: Res<SkillEvolution>,
    fusion: Res<HeroFusion>,
    assets: Res<AssetServer>,
    mut buttons: Query<(Entity, &Name, &mut Interaction), With<Button>>,
    children: Query<&Children>,
    images: Query<&ImageNode>,
    targets: Query<&Enemy, With<FusionTarget>>,
    names: Query<&Name>,
    session: Res<GameSession>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
) {
    state.frame += 1;
    let frame = state.frame;
    state.damage |= targets.iter().any(|enemy| enemy.hp < enemy.max_hp);
    if matches!(frame, 5 | 60) {
        recruitment.requested = true;
    }
    let mut chosen = None;
    if matches!(frame, 15 | 75) {
        let handles: Vec<_> = HeroKind::ALL
            .into_iter()
            .map(|kind| {
                (
                    kind,
                    assets.load(format!("ui/portraits/hero_{}.png", HEROES[kind.index()].id)),
                )
            })
            .collect();
        for (entity, name, _) in &buttons {
            if name.as_str().starts_with("Commander card ")
                && let Some(kind) = portrait_kind(entity, &children, &images, &handles)
                && !fusion.has(kind)
                && kind != selected.kind
            {
                chosen = Some(name.as_str().to_owned());
                state.candidate = Some(kind);
                break;
            }
        }
        assert!(
            chosen.is_some(),
            "recruitment must offer a new legacy identity"
        );
    }
    let action = chosen.as_deref().or(match frame {
        350 => Some("Hero attributes open"),
        _ => None,
    });
    if let Some(action) = action {
        let mut found = false;
        for (_, name, mut interaction) in &mut buttons {
            if name.as_str() == action {
                *interaction = Interaction::Pressed;
                found = true;
            }
        }
        assert!(found, "missing fusion control {action}");
    }
    if matches!(frame, 16 | 36 | 76 | 96 | 351) {
        for (_, _, mut interaction) in &mut buttons {
            *interaction = Interaction::None;
        }
    }
    if matches!(frame, 45 | 110) {
        assert_eq!(Some(selected.kind), state.candidate);
        assert!(
            state
                .before
                .iter()
                .zip(build.skills)
                .all(|(old, new)| new >= *old)
        );
        assert_eq!(state.tiers, evolution.tiers);
        let expected = if frame == 45 { 1 } else { 2 };
        assert_eq!(fusion.count(), expected);
        assert_eq!(evolution.skill_capacity(), fusion.skill_capacity());
        assert_eq!(fusion.skill_capacity(), 6 + expected);
        if frame == 45 {
            state.first = Some(HeroKind::StormSavant);
            state.second = Some(selected.kind);
        } else {
            assert!(fusion.has(state.second.unwrap()));
            assert_eq!(recruitment.fruit_count, 0);
        }
        state.before = build.skills;
        println!(
            "[capture] PASS: recruitment {expected}, unique inherited heroes={}, capacity={}, retained every old rank/tier",
            fusion.count(),
            evolution.skill_capacity()
        );
    }
    if frame == 340 {
        for kind in [state.first.unwrap(), state.second.unwrap()] {
            assert!(
                fusion.attacks_emitted[kind.index()] > 0,
                "no actual legacy attack from {kind:?}"
            );
        }
        assert!(state.damage);
        println!(
            "[capture] PASS: both inherited weapons emitted real combat attacks {:?}; enemies damaged",
            fusion.attacks_emitted
        );
    }
    if frame == 380 {
        assert!(session.paused);
        assert!(
            names
                .iter()
                .any(|name| name.as_str() == "Hero inherited abilities")
        );
        let window = windows.single().unwrap();
        let (_, node, transform) = layout
            .iter()
            .find(|(name, _, _)| name.as_str() == "Hero attributes panel")
            .unwrap();
        let min = transform.translation - node.size() * 0.5;
        let max = transform.translation + node.size() * 0.5;
        assert!(
            min.cmpge(Vec2::splat(-1.0)).all()
                && max
                    .cmple(Vec2::new(
                        window.physical_width() as f32 + 1.0,
                        window.physical_height() as f32 + 1.0
                    ))
                    .all()
        );
        for kind in [state.first.unwrap(), state.second.unwrap()] {
            assert!(
                names
                    .iter()
                    .any(|name| name.as_str()
                        == format!("Inherited hero {}", HEROES[kind.index()].id))
            );
        }
        println!(
            "[capture] PASS: both inherited hero portraits/weapon/ability rows available in bounded attribute sheet"
        );
    }
}
