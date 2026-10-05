//! Real paid UR draw, exclusive branch selection and runtime branch witnesses.
use super::CaptureControls;
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, DraftAction, DraftActionButton, DraftChoice, DraftState,
    Enemy, GameAssets, GameSession, Hero, HeroKind, HeroVitals, PlayerBuild, RecruitmentProgress,
    SelectedHero, SkillDrawButton, SkillId, SpawnDirector, UrBranchCue,
    skill_evolution::SkillEvolution, ur_evolution::UrEvolutionPath,
};
#[derive(Component)]
struct BranchTarget(Vec3);
#[derive(Resource)]
struct BranchCapture {
    frame: u32,
    skill: usize,
    path: UrEvolutionPath,
    cues: usize,
    damaged: bool,
    moved: bool,
    max_summons: usize,
    start_health: f32,
    pause: Vec<(Entity, Vec3, f32)>,
    pause_cues: usize,
    pause_wood: u32,
}
pub fn accepts(scenario: &str) -> bool {
    scenario == "ur-evolution" || (26..=29).any(|i| scenario.starts_with(&format!("ur{i}-")))
}
pub fn configure(world: &mut World, scenario: &str) {
    let (skill, path) = if scenario == "ur-evolution" {
        (26, UrEvolutionPath::Surge)
    } else {
        let (skill, path) = scenario
            .strip_prefix("ur")
            .unwrap()
            .split_once('-')
            .unwrap();
        let path = match path {
            "surge" => UrEvolutionPath::Surge,
            "control" => UrEvolutionPath::Control,
            "sustain" => UrEvolutionPath::Sustain,
            _ => panic!("unknown UR capture specialization"),
        };
        (skill.parse::<usize>().unwrap(), path)
    };
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CaptureControls>().suppress_drafts = false;
    world.resource_mut::<SelectedHero>().kind = HeroKind::StarWagerer;
    world.resource_mut::<SelectedHero>().locked = true;
    *world.resource_mut::<HeroVitals>() = HeroVitals::for_hero(HeroKind::StarWagerer, 50);
    world
        .resource_mut::<RecruitmentProgress>()
        .claimed_milestones = 7;
    world.resource_mut::<SpawnDirector>().enabled = false;
    let mut session = world.resource_mut::<GameSession>();
    session.level = 50;
    session.gold = 50_000;
    session.wood = 60;
    session.skill_points = 0;
    session.auto_bond_draws = false;
    world.insert_resource(BranchCapture {
        frame: 0,
        skill,
        path,
        cues: 0,
        damaged: false,
        moved: false,
        max_summons: 0,
        start_health: 0.,
        pause: Vec::new(),
        pause_cues: 0,
        pause_wood: 0,
    });
    let mut build = world.resource_mut::<PlayerBuild>();
    build.skills.fill(0);
    build.skills[skill] = 5;
}
pub fn install(app: &mut bevy::app::SubApp) {
    let world = app.world_mut();
    world.resource_mut::<GameSession>().skill_points = 0;
    {
        let mut economy = world.resource_mut::<rogue_commander::game::SkillEconomy>();
        economy.first_free_granted = true;
        economy.level_grants = u8::MAX;
    }

    world.remove_resource::<Autoplay>();
    let state = world.resource::<BranchCapture>();
    let (skill, path) = (state.skill, state.path);
    let mut build = world.resource_mut::<PlayerBuild>();
    build.skills.fill(0);
    build.skills[skill] = 5;
    if skill == 28 && path == UrEvolutionPath::Sustain {
        build.skills[4] = 1; // Real bone sentry casts supply the legion's firing units.
    }
    world.resource_mut::<SkillEvolution>().paths.fill(None);
    world.resource_mut::<ActiveSkill>().remaining = 10_000.;
    let mut attacks = world.resource_mut::<AttackClock>();
    attacks.0.reset();
    if skill == 28 && path == UrEvolutionPath::Surge {
        attacks.0.unpause();
    } else {
        attacks.0.pause();
    }
    world.resource_mut::<GameSession>().core_hp = 500.;
    let health = world.resource::<HeroVitals>().max_health * 0.5;
    world.resource_mut::<HeroVitals>().health = health;
    world.resource_mut::<BranchCapture>().start_health = health;
    for mut hero in world
        .query_filtered::<&mut Transform, With<Hero>>()
        .iter_mut(world)
    {
        hero.translation = Vec3::new(-4., 0., 0.);
    }
    for (mut camera, mut projection) in world
        .query_filtered::<(&mut Transform, &mut Projection), With<Camera3d>>()
        .iter_mut(world)
    {
        *camera = Transform::from_xyz(-1., 15., 12.).looking_at(Vec3::new(-1., 0., 0.), Vec3::Y);
        if let Projection::Orthographic(p) = &mut *projection {
            p.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 14.,
            };
        }
    }
    let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
    for (x, z) in [
        (-1., 0.),
        (0., 2.),
        (0., -2.),
        (2., 0.),
        (2., 3.),
        (2., -3.),
    ] {
        let position = Vec3::new(x, 0., z);
        world.spawn((
            Name::new("UR branch durable target"),
            BranchTarget(position),
            WorldAssetRoot(scene.clone()),
            Transform::from_translation(position).with_scale(Vec3::splat(0.7)),
            Enemy {
                hp: 100_000.,
                max_hp: 100_000.,
                speed: 0.,
                damage: 0.,
                xp: 0,
                wood: 0,
                gold: 0,
                kill_credit: 0,
                elite: false,
            },
        ));
    }
    app.add_systems(PreUpdate, controls.after(bevy::ui::UiSystems::Focus));
    app.add_systems(Last, audit);
    println!(
        "[capture] UR {skill} {path:?}: real draw → three routes → immediate click; 400 frames"
    );
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn controls(
    mut state: ResMut<BranchCapture>,
    mut session: ResMut<GameSession>,
    draft: Res<DraftState>,
    evolution: Res<SkillEvolution>,
    build: Res<PlayerBuild>,
    mut buttons: Query<
        (
            Option<&Name>,
            &mut Interaction,
            Has<SkillDrawButton>,
            Option<&DraftActionButton>,
        ),
        With<Button>,
    >,
) {
    state.frame += 1;
    let frame = state.frame;
    let mut action = None;
    if frame == 35 {
        assert_eq!(
            draft.choices.len(),
            3,
            "mastered UR exposes exactly three paths"
        );
        for path in UrEvolutionPath::ALL {
            assert!(draft.choices.contains(&DraftChoice::EvolveUr {
                skill: SkillId(state.skill as u8),
                path
            }));
        }
        let index = draft
            .choices
            .iter()
            .position(|c| {
                *c == DraftChoice::EvolveUr {
                    skill: SkillId(state.skill as u8),
                    path: state.path,
                }
            })
            .unwrap();
        action = Some(format!("Build card {index}"));
    }
    let mut pressed = false;
    for (name, mut interaction, skill, draft_action) in &mut buttons {
        if (matches!(frame, 10 | 330) && skill)
            || (matches!(frame, 46 | 350)
                && draft_action.is_some_and(|a| a.0 == DraftAction::Later))
            || action
                .as_ref()
                .is_some_and(|a| name.is_some_and(|n| n.as_str() == a))
        {
            *interaction = Interaction::Pressed;
            pressed = true;
        } else if matches!(frame, 11 | 36 | 47 | 331 | 351) {
            *interaction = Interaction::None;
        }
    }
    if matches!(frame, 10 | 35 | 46 | 330 | 350) {
        assert!(pressed, "UR UI action missing at frame {frame}");
    }
    if frame == 60 {
        assert_eq!(evolution.paths[state.skill], Some(state.path));
        assert!(
            !evolution.eligible(SkillId(state.skill as u8), &build),
            "UR cannot receive linear ascension"
        );
        assert!(
            !evolution.branch_ready(SkillId(state.skill as u8), &build),
            "chosen UR cannot offer another branch"
        );
        assert!(draft.root.is_none() && !session.paused);
    }
    if frame == 350 {
        assert!(draft.root.is_some() && session.paused);
        assert!(
            !draft.choices.iter().any(|choice| match choice {
                DraftChoice::EvolveUr { skill, .. } | DraftChoice::Evolution { skill, .. } =>
                    usize::from(skill.0) == state.skill,
                _ => false,
            }),
            "subsequent paid draw must not offer selected UR another path or linear ascension"
        );
    }
    if frame == 240 {
        session.paused = true;
    }
    if frame == 265 {
        session.paused = false;
    }
}
#[allow(clippy::too_many_arguments)]
fn audit(
    mut state: ResMut<BranchCapture>,
    mut cues: MessageReader<UrBranchCue>,
    enemies: Query<(Entity, &Transform, &Enemy, &BranchTarget)>,
    roots: Query<(&WorldAssetRoot, &Visibility)>,
    assets: Res<GameAssets>,
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    evolution: Res<SkillEvolution>,
) {
    for cue in cues.read() {
        if cue.skill == state.skill && cue.path == state.path {
            state.cues += 1;
        }
    }
    for (_, transform, enemy, initial) in &enemies {
        state.damaged |= enemy.hp < enemy.max_hp;
        state.moved |= transform.translation.distance(initial.0) > 0.1;
    }
    let summon_count = roots
        .iter()
        .filter(|(root, visibility)| {
            **visibility != Visibility::Hidden && assets.summon_scenes.contains(&root.0)
        })
        .count();
    state.max_summons = state.max_summons.max(summon_count);
    assert!(summon_count <= 32, "UR summons must stay bounded");
    if state.frame == 240 {
        state.pause = enemies
            .iter()
            .map(|(e, t, h, _)| (e, t.translation, h.hp))
            .collect();
        state.pause_cues = state.cues;
        state.pause_wood = session.wood;
    }
    if state.frame == 264 {
        assert_eq!(
            state.cues, state.pause_cues,
            "paused branch must not activate"
        );
        assert_eq!(session.wood, state.pause_wood);
        for (entity, position, hp) in &state.pause {
            let (_, transform, enemy, _) = enemies.get(*entity).unwrap();
            assert_eq!(transform.translation, *position);
            assert_eq!(enemy.hp, *hp);
        }
    }
    if state.frame == 395 {
        assert!(
            state.cues > 0,
            "selected UR branch never actually activated"
        );
        assert_eq!(evolution.paths[state.skill], Some(state.path));
        match (state.skill, state.path) {
            (26, UrEvolutionPath::Sustain) => assert!(state.max_summons >= 2 && state.damaged),
            (27, UrEvolutionPath::Control) => assert!(
                state.moved && state.damaged,
                "gravity must pull and damage actual enemies"
            ),
            (27, UrEvolutionPath::Sustain) => assert!(
                vitals.health > state.start_health && session.core_hp > 500.,
                "guardian must heal hero and gate"
            ),
            (28, UrEvolutionPath::Sustain) | (29, UrEvolutionPath::Surge) => {
                assert!(state.max_summons >= 1 && state.damaged)
            }
            (29, UrEvolutionPath::Sustain) => assert!(
                session.wood >= 68,
                "fortune must pay actual resources on repeated ticks"
            ),
            _ => assert!(state.damaged, "offensive route must deal actual damage"),
        }
        println!(
            "[capture] PASS UR {} {:?}: real 3-way selection exclusive; actions={} damage={} pull={} summons_max={} wood={} core={} pause frozen",
            state.skill,
            state.path,
            state.cues,
            state.damaged,
            state.moved,
            state.max_summons,
            session.wood,
            session.core_hp
        );
    }
}
