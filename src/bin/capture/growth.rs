//! Exercises late progression through the real draw/preview/confirm controls.
use bevy::prelude::*;
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, BondCardId, BondDrawButton, BondId, DraftChoice,
    DraftState, Enemy, GameAssets, GameSession, Hero, PlayerBuild, RecruitmentProgress,
    SelectedHero, SkillDrawButton, SpawnDirector, TrialState, available_bond_cards_at,
    skill_evolution::SkillEvolution,
};
#[derive(Resource, Default)]
struct GrowthCapture {
    frame: u32,
    cultivation: bool,
    ray: bool,
    mine: bool,
    damage: bool,
    notice: bool,
    chosen: Option<DraftChoice>,
}
#[derive(Component)]
struct GrowthTarget;
pub fn configure(world: &mut World, scenario: &str) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world
        .resource_mut::<RecruitmentProgress>()
        .claimed_milestones = 7;
    world.resource_mut::<SpawnDirector>().enabled = false;
    let mut session = world.resource_mut::<GameSession>();
    session.level = 50;
    session.elapsed = 1201.0;
    session.gold = 50_000;
    session.wood = 1000;
    session.skill_points = 0;
    session.auto_bond_draws = false;
    if scenario == "growth-cultivation" {
        let mut build = world.resource_mut::<PlayerBuild>();
        for card in BondCardId::ALL {
            build.owned_bond_cards[card.index()] =
                !matches!(card.family(), BondId::SpiritEnergy | BondId::MadScientist);
        }
        let last = *BondId::Cultivation.definition().members.last().unwrap();
        build.owned_bond_cards[last.index()] = false;
        assert_eq!(build.bond(BondId::Cultivation), 7);
    }
    // Seed mastery before the warm frame snapshots feedback state. Only the
    // real UI-confirmed ascension should generate a training notice.
    world.resource_mut::<PlayerBuild>().skills.fill(0);
    if scenario != "growth-cultivation" {
        for index in [4, 6, 16, 18, 30, 31] {
            world.resource_mut::<PlayerBuild>().skills[index] = 5;
        }
        world.resource_mut::<SkillEvolution>().tiers[4] = 3;
        world.resource_mut::<SkillEvolution>().tiers[6] = 3;
    }
    world.insert_resource(GrowthCapture {
        cultivation: scenario == "growth-cultivation",
        ..default()
    });
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
    world.resource_mut::<ActiveSkill>().remaining = 10_000.0;
    world.resource_mut::<AttackClock>().0.pause();
    world.resource_mut::<TrialState>().level = 12;
    let cultivation = world.resource::<GrowthCapture>().cultivation;
    if !cultivation {
        let scene = world.resource::<GameAssets>().enemy_scenes[0].clone();
        for position in [
            Vec3::new(-1.5, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(3.5, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
        ] {
            world.spawn((
                Name::new("Late progression durable target"),
                GrowthTarget,
                WorldAssetRoot(scene.clone()),
                Transform::from_translation(position).with_scale(Vec3::splat(0.7)),
                Enemy {
                    hp: 100_000.0,
                    max_hp: 100_000.0,
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
    }
    let center = Vec3::new(-4.0, 0.0, 0.0);
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
        *camera = Transform::from_translation(Vec3::new(-1.0, 14.0, 11.0))
            .looking_at(Vec3::new(-1.0, 0.0, 0.0), Vec3::Y);
        if let Projection::Orthographic(projection) = &mut *projection {
            projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
                viewport_height: 13.0,
            };
        }
    }
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
    println!(
        "[capture] growth: real paid draw controls; six rank-five skills or seven of eight cultivation cards"
    );
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn exercise(
    mut state: ResMut<GrowthCapture>,
    mut buttons: Query<
        (
            Option<&Name>,
            &mut Interaction,
            Has<SkillDrawButton>,
            Has<BondDrawButton>,
        ),
        With<Button>,
    >,
    draft: Res<DraftState>,
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    evolution: Res<SkillEvolution>,
    names: Query<&Name>,
    targets: Query<&Enemy, With<GrowthTarget>>,
) {
    state.frame += 1;
    state.ray |= names
        .iter()
        .any(|n| n.as_str() == "Flame ray sweeping beam");
    state.mine |= names.iter().any(|n| n.as_str() == "Goblin mine armed trap");
    state.notice |= names.iter().any(|n| n.as_str() == "Skill upgrade notice");
    state.damage |= targets.iter().any(|enemy| enemy.hp < enemy.max_hp);
    let frame = state.frame;
    let draw = matches!(frame, 10 | 170);
    let select = matches!(frame, 35 | 195);
    let mut action = None;
    if select {
        let index = draft
            .choices
            .iter()
            .position(|choice| {
                if state.cultivation {
                    matches!(choice, DraftChoice::Bond(_))
                } else if frame == 35 {
                    matches!(choice, DraftChoice::Evolution { .. })
                } else {
                    matches!(choice, DraftChoice::ReplaceSkill { .. })
                }
            })
            .expect("required late progression choice is offered");
        state.chosen = Some(draft.choices[index]);
        action = Some(format!("Build card {index}"));
    }
    let release = matches!(frame, 11 | 36 | 46 | 171 | 196 | 206);
    let mut pressed = false;
    for (name, mut interaction, skill, bond) in &mut buttons {
        if (draw && if state.cultivation { bond } else { skill })
            || action
                .as_ref()
                .is_some_and(|action| name.is_some_and(|name| name.as_str() == action))
        {
            *interaction = Interaction::Pressed;
            pressed = true;
        } else if release {
            *interaction = Interaction::None;
        }
    }
    if draw || select {
        assert!(
            pressed,
            "required growth UI control missing at frame {frame}"
        );
    }
    if frame == 25 {
        assert!(session.paused && draft.root.is_some());
        if state.cultivation {
            assert_eq!(draft.choices.len(), 1);
        } else {
            assert_eq!(build.skills.iter().filter(|rank| **rank > 0).count(), 6);
            assert!(
                draft
                    .choices
                    .iter()
                    .any(|c| matches!(c, DraftChoice::Evolution { .. }))
            );
            assert!(
                draft
                    .choices
                    .iter()
                    .any(|c| matches!(c, DraftChoice::ReplaceSkill { .. }))
            );
        }
        println!("[capture] PASS: late draw offers {:?}", draft.choices);
    }
    if frame == 65 {
        if state.cultivation {
            assert!(build.bond_complete(BondId::Cultivation));
            let pool = available_bond_cards_at(&build, &session);
            assert_eq!(pool.len(), 12);
            assert!(
                pool.iter()
                    .all(|card| card.family() == BondId::SpiritEnergy)
            );
            println!(
                "[capture] PASS: actual confirmation completed cultivation 8/8 and unlocked all 12 spirit-energy cards"
            );
        } else if let Some(DraftChoice::Evolution { skill, tier }) = state.chosen {
            assert_eq!(evolution.tiers[usize::from(skill.0)], tier);
            assert_eq!(build.skill_rank(skill), 5);
            assert_eq!(build.skills.iter().filter(|rank| **rank > 0).count(), 6);
            println!(
                "[capture] PASS: mastered skill {:?} ascended to {tier}; six occupied slots retained",
                skill
            );
        } else {
            panic!("first training confirmation was not an evolution");
        }
    }
    if frame == 225 && !state.cultivation {
        let Some(DraftChoice::ReplaceSkill { old, new }) = state.chosen else {
            panic!("expected explicit replacement");
        };
        assert_eq!(build.skill_rank(old), 0);
        assert_eq!(build.skill_rank(new), 1);
        assert_eq!(build.skills.iter().filter(|rank| **rank > 0).count(), 6);
        println!(
            "[capture] PASS: explicit UR replacement {:?} -> {:?}; six slots, old skill removed",
            old, new
        );
    }
    if frame == 295 {
        if !state.cultivation {
            assert!(
                state.ray && state.mine && state.damage && state.notice,
                "beam={},mine={},damage={},notice={}",
                state.ray,
                state.mine,
                state.damage,
                state.notice
            );
            println!(
                "[capture] PASS: real flame beam and armed mine spawned, enemies damaged, ascension toast displayed"
            );
        } else {
            assert_eq!(build.bond(BondId::SpiritEnergy), 1);
            println!(
                "[capture] PASS: first spirit-energy card acquired through actual draw and confirmation"
            );
        }
    }
}
