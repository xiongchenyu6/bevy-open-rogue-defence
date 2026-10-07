use super::skill_evolution::SkillEvolution;
use super::{HeroFusion, SkillId};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::combat::SurvivalSet;
#[cfg(test)]
use super::content::Attribute;
use super::content::{ContentId, HEROES, Rarity, SKILLS};
use super::localization::hero_name;
use super::{
    Autoplay, GameAssets, GameLanguage, GameSession, Hero, HeroIdentity, HeroKind,
    HeroProgressionDelta, HeroVisual, HeroVitals, PlayerBuild, RecruitmentProgress,
    RecruitmentSource, RunRng, RunSeed, SelectedHero,
};

#[path = "hero_draft.rs"]
pub(super) mod draft;
use draft::spawn_hero_selector;

pub struct HeroPlugin;

const HERO_OFFER_COUNT: usize = 6;
const HERO_OFFER_SEED_SALT: u64 = 0x4845_524F_5F4F_4646;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLanguage>()
            .init_resource::<HeroState>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<HeroFusion>()
            .add_systems(Startup, (setup_hero_ui, draft::load_art))
            .add_systems(
                Update,
                (
                    open_hero_selector,
                    draft::update_layout,
                    handle_hero_choice,
                    draft::update_feedback,
                    draft::refresh_language,
                    refresh_hero_language,
                    update_hero_selector_panel_visibility,
                    synchronize_hero_growth,
                    trigger_recruitment,
                    update_hero_status,
                )
                    .chain()
                    .before(SurvivalSet),
            );
    }
}

#[derive(Resource)]
struct HeroState {
    selected: HeroKind,
    preview: Option<HeroOffer>,
    locked: bool,
    root: Option<Entity>,
    offer_generation: u32,
}

impl Default for HeroState {
    fn default() -> Self {
        Self {
            selected: HeroKind::StormSavant,
            preview: None,
            locked: false,
            root: None,
            offer_generation: 0,
        }
    }
}

#[derive(Component)]
struct HeroSelectButton;

#[derive(Component)]
struct HeroSelectorStatusPanel;

#[derive(Component)]
struct HeroSelectorRoot;

#[derive(Component)]
pub(super) struct HeroChoiceButton(HeroOffer);
impl HeroChoiceButton {
    pub(super) fn kind(&self) -> HeroKind {
        self.0.kind
    }
}
#[derive(Component)]
struct HeroChoiceOrdinal(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeroOffer {
    kind: HeroKind,
    starting_skill: ContentId,
}

#[derive(Component)]
struct HeroStatusText;

fn setup_hero_ui(mut commands: Commands, language: Res<GameLanguage>) {
    let status = format!(
        "[R] {} · {} 0/3",
        hero_name(*language, HeroKind::StormSavant),
        language.pick("RECRUIT", "招募")
    );
    let choose_hero = language.pick("CHOOSE HERO", "选择英雄");
    commands
        .spawn((
            HeroSelectorStatusPanel,
            Node {
                position_type: PositionType::Absolute,
                left: percent(43),
                top: px(64),
                width: px(168),
                padding: UiRect::all(px(8)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.055, 0.09, 0.82)),
            BorderColor::all(Color::srgba(0.2, 0.66, 0.85, 0.5)),
            GlobalZIndex(16),
            Visibility::Inherited,
        ))
        .with_children(|panel| {
            panel.spawn((
                HeroStatusText,
                Text::new(status),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.62, 0.9, 1.0)),
            ));
            panel
                .spawn((
                    Button,
                    HeroSelectButton,
                    Node {
                        width: percent(100),
                        height: px(48),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border_radius: BorderRadius::all(px(7)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.08, 0.23, 0.34)),
                ))
                .with_children(|button| {
                    button.spawn((
                        HeroStaticCopy::Choose,
                        Text::new(choose_hero),
                        TextFont {
                            font_size: FontSize::Px(12.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
        });
}

fn open_hero_selector(
    mut commands: Commands,
    autoplay: Option<Res<Autoplay>>,
    campaign_menu: Option<Res<super::campaign_ui::CampaignMenu>>,
    art: Res<draft::DraftArt>,
    run_seed: Res<RunSeed>,
    language: Res<GameLanguage>,
    selected_hero: Res<SelectedHero>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<HeroState>,
) {
    if campaign_menu.is_some_and(|menu| menu.0)
        || autoplay.is_some()
        || selected_hero.locked
        || state.locked
        || state.root.is_some()
        || session.paused
        || session.finished
    {
        return;
    }
    session.paused = true;
    let offers = roll_hero_offers(run_seed.0, state.offer_generation);
    state.preview = Some(offers[0]);
    state.root = Some(spawn_hero_selector(&mut commands, &offers, &art, *language));
}

fn update_hero_selector_panel_visibility(
    selected_hero: Res<SelectedHero>,
    mut panels: Query<&mut Visibility, With<HeroSelectorStatusPanel>>,
) {
    let visibility = if selected_hero.locked {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut panel_visibility in &mut panels {
        *panel_visibility = visibility;
    }
}

#[derive(SystemParam)]
struct RecruitmentInheritance<'w> {
    fusion: ResMut<'w, HeroFusion>,
    evolution: Option<ResMut<'w, SkillEvolution>>,
}

#[derive(SystemParam)]
struct HeroChoiceInput<'w> {
    keyboard: Option<ResMut<'w, ButtonInput<KeyCode>>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    touches: Option<Res<'w, Touches>>,
    readiness: Option<Res<'w, super::AssetReadiness>>,
    tutorial: Option<Res<'w, super::OnboardingState>>,
}
fn handle_hero_choice(
    mut commands: Commands,
    interactions: Query<(&Interaction, &HeroChoiceButton), Changed<Interaction>>,
    ordinals: Query<(&HeroChoiceButton, &HeroChoiceOrdinal)>,
    mut input: HeroChoiceInput,
    assets: Option<Res<GameAssets>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut state: ResMut<HeroState>,
    mut selected_hero: ResMut<SelectedHero>,
    mut identities: Query<&mut HeroIdentity, With<Hero>>,
    mut visuals: Query<&mut WorldAssetRoot, With<HeroVisual>>,
    mut progress: ResMut<RecruitmentProgress>,
    mut runtime: Option<ResMut<super::bonds::BondRuntime>>,
    autoplay: Option<Res<Autoplay>>,
    mut inheritance: RecruitmentInheritance,
) {
    if input
        .tutorial
        .as_ref()
        .is_some_and(|guide| guide.input_blocked)
    {
        return;
    }
    if state.root.is_none() || input.readiness.as_ref().is_some_and(|gate| !gate.ready) {
        return;
    }
    if input.readiness.as_ref().is_some_and(|gate| {
        ordinals
            .iter()
            .any(|(button, _)| !gate.heroes.contains(&button.kind()))
    }) {
        return;
    }
    let held = input
        .mouse
        .as_ref()
        .is_some_and(|mouse| mouse.pressed(MouseButton::Left))
        || input
            .touches
            .as_ref()
            .is_some_and(|touches| touches.iter().next().is_some());
    let fresh = input
        .mouse
        .as_ref()
        .is_some_and(|mouse| mouse.just_pressed(MouseButton::Left))
        || input
            .touches
            .as_ref()
            .is_some_and(|touches| touches.iter_just_pressed().next().is_some());
    let mut clicked = interactions.iter().find_map(|(interaction, choice)| {
        if *interaction == Interaction::Hovered {
            state.preview = Some(choice.0);
        }
        (*interaction == Interaction::Pressed && (!held || fresh)).then_some(choice.0)
    });
    if let Some(keys) = input.keyboard.as_deref_mut()
        && let Some(index) = [
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
        ]
        .iter()
        .position(|key| keys.just_pressed(*key))
    {
        clicked = ordinals
            .iter()
            .find(|(_, ordinal)| ordinal.0 == index)
            .map(|(choice, _)| choice.0);
        keys.clear();
    }
    if let Some(offer) = clicked {
        state.preview = Some(offer);
    }
    if clicked.is_none() && !(autoplay.is_some() && progress.pending.is_some()) {
        return;
    }
    let Some(offer) = state.preview else {
        return;
    };
    if let Some(source) = progress.pending {
        if offer.kind == selected_hero.kind || !progress.claim(source) {
            return;
        }
        inheritance.fusion.record_absorbed(vitals.kind);
        let missing_health = vitals.max_health - vitals.health;
        let missing_energy = vitals.max_energy - vitals.energy;
        if let Some(runtime) = runtime.as_deref_mut() {
            runtime.remove_applied(&mut build, &mut vitals);
        }
        absorb_and_recruit(
            offer.kind,
            session.level,
            &mut progress,
            &mut build,
            &mut vitals,
        );
        if let Some(runtime) = runtime.as_deref_mut() {
            runtime.apply_stats(&mut build, &mut vitals);
        }
        vitals.health = (vitals.max_health - missing_health).clamp(0., vitals.max_health);
        vitals.energy = (vitals.max_energy - missing_energy).clamp(0., vitals.max_energy);
    } else {
        if selected_hero.locked || HEROES[offer.kind.index()].rarity != Rarity::R {
            return;
        }
        apply_hero_stats(
            offer.kind,
            offer.starting_skill,
            session.level,
            &mut build,
            &mut vitals,
        );
    }
    let skill = inheritance
        .fusion
        .starting_skill(offer.kind)
        .unwrap_or_else(|| {
            SkillId(
                SKILLS
                    .iter()
                    .position(|skill| skill.id == offer.starting_skill)
                    .expect("valid starting skill") as u8,
            )
        });
    inheritance
        .fusion
        .register_starting_skill(offer.kind, skill);
    let index = usize::from(skill.0);
    build.skills[index] = build.skills[index].max(
        HEROES[offer.kind.index()]
            .rarity
            .starting_skill_rank()
            .min(SKILLS[index].max_rank),
    );
    if let Some(evolution) = inheritance.evolution.as_deref_mut() {
        evolution.rewarded_ranks[index] = evolution.rewarded_ranks[index].max(build.skills[index]);
    }
    state.selected = offer.kind;
    state.locked = true;
    selected_hero.kind = offer.kind;
    selected_hero.locked = true;
    for mut identity in &mut identities {
        identity.0 = offer.kind;
    }
    if let Some(assets) = assets.as_deref() {
        for mut scene in &mut visuals {
            scene.0 = assets.hero_scenes[state.selected.index()].clone();
        }
    }
    close_overlay(&mut commands, &mut session, &mut state);
}

fn trigger_recruitment(
    mut commands: Commands,
    art: Res<draft::DraftArt>,
    run_seed: Res<RunSeed>,
    language: Res<GameLanguage>,
    selected: Res<SelectedHero>,
    fusion: Res<HeroFusion>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<HeroState>,
    mut progress: ResMut<RecruitmentProgress>,
    vitals: Res<HeroVitals>,
) {
    if !selected.locked
        || state.root.is_some()
        || session.paused
        || session.finished
        || !vitals.can_act()
        || vitals.health <= 0.
    {
        return;
    }
    let source = progress
        .next_milestone(session.level)
        .map(RecruitmentSource::Milestone)
        .or_else(|| {
            (progress.requested && progress.fruit_count > 0).then_some(RecruitmentSource::Fruit)
        });
    progress.requested = false;
    let Some(source) = source else {
        return;
    };
    progress.pending = Some(source);
    state.offer_generation = state.offer_generation.saturating_add(1);
    let offers =
        roll_recruitment_offers(run_seed.0, state.offer_generation, selected.kind, &fusion);
    state.preview = Some(offers[0]);
    session.paused = true;
    state.root = Some(spawn_hero_selector(&mut commands, &offers, &art, *language));
}

fn absorb_and_recruit(
    kind: HeroKind,
    level: u32,
    progress: &mut RecruitmentProgress,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
) {
    let old = HeroVitals::for_hero(vitals.kind, level);
    progress.absorbed_strength += old.strength;
    progress.absorbed_agility += old.agility;
    progress.absorbed_intelligence += old.intelligence;
    let next = HeroVitals::for_hero(kind, level);
    let primary_before = vitals.primary_attribute();
    let old_agility = vitals.agility;
    let old_intelligence = vitals.intelligence;
    // Replace intrinsic stats and retain all independently accumulated bonuses.
    vitals.strength += next.strength;
    vitals.agility += next.agility;
    vitals.intelligence += next.intelligence;
    let health_delta =
        (next.max_health - old.max_health + old.strength * 14.) * vitals.command_health_multiplier;
    let energy_delta = next.max_energy - old.max_energy + old.intelligence * 8.;
    vitals.max_health += health_delta;
    vitals.health = (vitals.health + health_delta).clamp(0., vitals.max_health);
    vitals.max_energy += energy_delta;
    vitals.energy = (vitals.energy + energy_delta).clamp(0., vitals.max_energy);
    vitals.kind = kind;
    vitals.level = level;
    apply_growth_delta(
        build,
        HeroProgressionDelta {
            primary: vitals.primary_attribute() - primary_before,
            agility: vitals.agility - old_agility,
            intelligence: vitals.intelligence - old_intelligence,
            max_health: health_delta,
            max_energy: energy_delta,
        },
    );
}

pub(crate) fn synchronize_hero_growth(
    selected: Res<SelectedHero>,
    session: Res<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut runtime: Option<ResMut<super::bonds::BondRuntime>>,
) {
    let before_health = vitals.health;
    let before_max = vitals.max_health;
    let before_energy = vitals.energy;
    let before_max_energy = vitals.max_energy;
    let survival = (
        vitals.alive,
        vitals.deaths,
        vitals.revive_remaining,
        vitals.invulnerable_remaining,
        vitals.stunned_remaining,
    );
    if let Some(runtime) = runtime.as_deref_mut() {
        runtime.remove_applied(&mut build, &mut vitals);
    }
    if vitals.kind != selected.kind {
        let starting_skill = HEROES[selected.kind.index()].starting_skill;
        apply_hero_stats(
            selected.kind,
            starting_skill,
            session.level,
            &mut build,
            &mut vitals,
        );
    } else {
        let delta = vitals.synchronize_level(selected.kind, session.level);
        apply_growth_delta(&mut build, delta);
    }
    (
        vitals.alive,
        vitals.deaths,
        vitals.revive_remaining,
        vitals.invulnerable_remaining,
        vitals.stunned_remaining,
    ) = survival;
    if let Some(runtime) = runtime.as_deref_mut() {
        runtime.apply_stats(&mut build, &mut vitals);
    }
    if vitals.alive {
        vitals.health = if before_health <= 0.0 {
            0.0
        } else {
            (before_health + vitals.max_health - before_max).clamp(0.0, vitals.max_health)
        };
        vitals.energy =
            (before_energy + vitals.max_energy - before_max_energy).clamp(0.0, vitals.max_energy);
        if let Some(runtime) = runtime.as_deref_mut() {
            vitals.heal(runtime.take_kill_healing());
        }
    }
}

fn apply_growth_delta(build: &mut PlayerBuild, delta: HeroProgressionDelta) {
    build.attack_damage += delta.primary;
    build.attacks_per_second += delta.agility * 0.028;
    build.move_speed += delta.agility * 0.035;
    build.projectile_speed += delta.intelligence * 0.08;
    build.attack_range += delta.intelligence * 0.025;
}

pub fn starting_basic_attack_rate(agility: f32) -> f32 {
    2.1 + agility * 0.028
}

fn apply_hero_stats(
    kind: HeroKind,
    starting_skill: ContentId,
    level: u32,
    build: &mut PlayerBuild,
    vitals: &mut HeroVitals,
) {
    vitals.reset_for_hero(kind, level);
    build.attack_damage = vitals.attack_damage();
    build.attacks_per_second = starting_basic_attack_rate(vitals.agility);
    build.move_speed = 5.4 + vitals.agility * 0.035;
    build.projectile_speed = 17.0 + vitals.intelligence * 0.08;
    build.attack_range = 8.5 + vitals.intelligence * 0.025;
    build.skills.fill(0);
    if let Some(skill_index) = SKILLS.iter().position(|skill| skill.id == starting_skill) {
        build.skills[skill_index] = HEROES[kind.index()]
            .rarity
            .starting_skill_rank()
            .min(SKILLS[skill_index].max_rank);
    }
}

fn update_hero_status(
    selected: Res<SelectedHero>,
    progress: Res<RecruitmentProgress>,
    language: Res<GameLanguage>,
    mut text: Query<&mut Text, With<HeroStatusText>>,
) {
    if let Ok(mut text) = text.single_mut() {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(format!(
                "[{}] {} · {} {}/3",
                HEROES[selected.kind.index()].rarity.label(),
                hero_name(*language, selected.kind),
                language.pick("RECRUIT", "招募"),
                progress.claimed_milestones.count_ones()
            ));
    }
}

fn close_overlay(commands: &mut Commands, session: &mut GameSession, state: &mut HeroState) {
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    session.paused = false;
}

fn roll_hero_offers(seed: u64, generation: u32) -> Vec<HeroOffer> {
    roll_offers(seed, generation, None)
}
fn roll_recruitment_offers(
    seed: u64,
    generation: u32,
    current: HeroKind,
    fusion: &HeroFusion,
) -> Vec<HeroOffer> {
    let mut offers = roll_offers(seed, generation, Some(current));
    for offer in &mut offers {
        if let Some(skill) = fusion.starting_skill(offer.kind) {
            offer.starting_skill = SKILLS[usize::from(skill.0)].id;
        }
    }
    offers
}
fn roll_offers(seed: u64, generation: u32, current: Option<HeroKind>) -> Vec<HeroOffer> {
    let mut rng = RunRng::from_seed(
        seed ^ HERO_OFFER_SEED_SALT
            ^ (u64::from(generation) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
    );
    let mut pool: Vec<_> = HeroKind::ALL
        .into_iter()
        .filter(|kind| {
            Some(*kind) != current
                && (current.is_some() || HEROES[kind.index()].rarity == Rarity::R)
        })
        .collect();
    let mut offers = Vec::new();
    while !pool.is_empty() && offers.len() < HERO_OFFER_COUNT {
        let kind = take_weighted_hero(&mut pool, &mut rng);
        let skills = HEROES[kind.index()].starting_skill_pool;
        offers.push(HeroOffer {
            kind,
            starting_skill: skills[rng.next_u32() as usize % skills.len()],
        });
    }
    offers
}

fn take_weighted_hero(pool: &mut Vec<HeroKind>, rng: &mut RunRng) -> HeroKind {
    let total_weight: u32 = pool
        .iter()
        .map(|kind| HEROES[kind.index()].rarity.draw_weight())
        .sum();
    let index = weighted_hero_index(pool, rng.next_u32() % total_weight.max(1));
    pool.swap_remove(index)
}

fn weighted_hero_index(pool: &[HeroKind], mut roll: u32) -> usize {
    pool.iter()
        .position(|kind| {
            let weight = HEROES[kind.index()].rarity.draw_weight();
            if roll < weight {
                true
            } else {
                roll -= weight;
                false
            }
        })
        .unwrap_or(pool.len().saturating_sub(1))
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(HeroState::default());
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn recruitment_app(level: u32) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<HeroState>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<HeroFusion>()
            .init_resource::<GameLanguage>()
            .init_resource::<draft::DraftArt>()
            .init_resource::<RunSeed>()
            .init_resource::<SelectedHero>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .insert_resource(HeroVitals::for_hero(HeroKind::StormSavant, level))
            .add_systems(Update, (handle_hero_choice, trigger_recruitment).chain());
        app.world_mut().resource_mut::<SelectedHero>().locked = true;
        app.world_mut().resource_mut::<GameSession>().level = level;
        app
    }

    fn click_recruit(app: &mut App) {
        let root = app.world().resource::<HeroState>().root.unwrap();
        let offer = app.world().resource::<HeroState>().preview.unwrap();
        let button = app
            .world_mut()
            .spawn((HeroChoiceButton(offer), Interaction::Pressed))
            .id();
        app.world_mut().entity_mut(root).add_child(button);
        app.update();
    }

    #[test]
    fn loading_gate_blocks_recruitment_button_and_keyboard_until_ready() {
        let mut app = recruitment_app(10);
        app.update();
        let old = app.world().resource::<SelectedHero>().kind;
        app.insert_resource(super::super::AssetReadiness::default());
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Digit1);
        app.insert_resource(keys);
        click_recruit(&mut app);
        assert_eq!(app.world().resource::<SelectedHero>().kind, old);
        assert!(app.world().resource::<HeroState>().root.is_some());
        assert!(app.world().resource::<GameSession>().paused);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .clear();
        let heroes: Vec<_> = app
            .world_mut()
            .query::<&HeroChoiceButton>()
            .iter(app.world())
            .map(|button| button.kind())
            .collect();
        {
            let mut gate = app
                .world_mut()
                .resource_mut::<super::super::AssetReadiness>();
            gate.ready = true;
            gate.heroes = heroes;
        }
        click_recruit(&mut app);
        assert_ne!(app.world().resource::<SelectedHero>().kind, old);
    }
    #[test]
    fn milestone_recruitment_queues_ten_twenty_forty_once_each() {
        let mut app = recruitment_app(40);
        app.update();
        for level in [10, 20, 40] {
            assert_eq!(
                app.world().resource::<RecruitmentProgress>().pending,
                Some(RecruitmentSource::Milestone(level))
            );
            let old = app.world().resource::<SelectedHero>().kind;
            assert_ne!(
                app.world().resource::<HeroState>().preview.unwrap().kind,
                old
            );
            click_recruit(&mut app);
        }
        let progress = app.world().resource::<RecruitmentProgress>();
        assert_eq!(progress.claimed_milestones, 0b111);
        assert!(progress.pending.is_none());
        app.update();
        assert!(app.world().resource::<HeroState>().root.is_none());
    }

    #[test]
    fn real_recruitment_inherits_skills_and_evolution_when_six_slots_are_full() {
        let mut app = recruitment_app(10);
        app.init_resource::<SkillEvolution>();
        app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
        app.world_mut().resource_mut::<PlayerBuild>().skills[..6].fill(5);
        app.world_mut().resource_mut::<SkillEvolution>().tiers[0] = 3;
        app.update();
        let offer = HeroOffer {
            kind: HeroKind::GroveArtificer,
            starting_skill: "bolt_turret",
        };
        let index = SKILLS
            .iter()
            .position(|skill| skill.id == offer.starting_skill)
            .unwrap();
        assert!(index >= 6);
        app.world_mut().resource_mut::<HeroState>().preview = Some(offer);
        click_recruit(&mut app);
        let build = app.world().resource::<PlayerBuild>();
        assert!(build.skills[..6].iter().all(|rank| *rank == 5));
        assert!(build.skills[index] > 0);
        assert_eq!(build.skills.iter().filter(|rank| **rank > 0).count(), 7);
        assert_eq!(app.world().resource::<SkillEvolution>().tiers[0], 3);
        assert!(
            app.world()
                .resource::<HeroFusion>()
                .has(HeroKind::StormSavant)
        );
        assert_eq!(app.world().resource::<HeroFusion>().count(), 1);
        assert_eq!(
            app.world()
                .resource::<HeroFusion>()
                .starting_skill(offer.kind),
            Some(SkillId(index as u8))
        );
        assert_eq!(
            app.world().resource::<SkillEvolution>().rewarded_ranks[index],
            build.skills[index]
        );
    }

    #[test]
    fn repeated_hero_offers_reuse_first_skill_instead_of_gifting_new_slots() {
        let mut fusion = HeroFusion::default();
        let kind = HeroKind::GroveArtificer;
        let index = SKILLS
            .iter()
            .position(|skill| skill.id == "bolt_turret")
            .unwrap();
        fusion.register_starting_skill(kind, SkillId(index as u8));
        let mut seen = 0;
        for seed in 1..50 {
            for offer in roll_recruitment_offers(seed, 1, HeroKind::StormSavant, &fusion) {
                if offer.kind == kind {
                    assert_eq!(offer.starting_skill, "bolt_turret");
                    seen += 1;
                }
            }
        }
        assert!(seen > 0);
    }

    #[test]
    fn fruit_is_spent_on_card_click_and_retains_build_and_missing_health() {
        let mut app = recruitment_app(1);
        {
            let mut progress = app.world_mut().resource_mut::<RecruitmentProgress>();
            progress.fruit_count = 1;
            progress.requested = true;
        }
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.skills[7] = 3;
            build.multishot = 4;
            build.pierce = 2;
            build.owned_bond_cards[0] = true;
        }
        let before_skills = app.world().resource::<PlayerBuild>().skills;
        let before_cards = app.world().resource::<PlayerBuild>().owned_bond_cards;
        app.world_mut().resource_mut::<HeroVitals>().health -= 123.;
        app.update();
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 1);
        assert!(app.world().resource::<GameSession>().paused);
        click_recruit(&mut app);
        let progress = app.world().resource::<RecruitmentProgress>();
        assert_eq!(progress.fruit_count, 0);
        let original = HeroVitals::for_hero(HeroKind::StormSavant, 1);
        assert_eq!(
            (
                progress.absorbed_strength,
                progress.absorbed_agility,
                progress.absorbed_intelligence
            ),
            (original.strength, original.agility, original.intelligence)
        );
        let vitals = app.world().resource::<HeroVitals>();
        assert!((vitals.max_health - vitals.health - 123.).abs() < 0.001);
        let build = app.world().resource::<PlayerBuild>();
        assert!(
            build
                .skills
                .iter()
                .zip(before_skills)
                .all(|(after, before)| *after >= before)
        );
        assert_eq!(build.owned_bond_cards, before_cards);
        assert_eq!((build.multishot, build.pierce), (4, 2));
    }

    #[test]
    fn swallowing_changes_identity_and_base_health_without_regranting_skill_slots() {
        let mut progress = RecruitmentProgress::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::for_hero(HeroKind::StormSavant, 20);
        apply_hero_stats(
            HeroKind::StormSavant,
            HEROES[HeroKind::StormSavant.index()].starting_skill,
            20,
            &mut build,
            &mut vitals,
        );
        build.skills.fill(0);
        build.skills[..6].fill(2);
        let skills = build.skills;
        build.attack_damage += 73.5;
        vitals.health -= 321.;
        for kind in [HeroKind::StoneWarden, HeroKind::EmberRanger] {
            absorb_and_recruit(kind, 20, &mut progress, &mut build, &mut vitals);
            let intrinsic = HeroVitals::for_hero(kind, 20);
            assert!(
                (vitals.strength - intrinsic.strength - progress.absorbed_strength).abs() < 0.001
            );
            assert!((vitals.agility - intrinsic.agility - progress.absorbed_agility).abs() < 0.001);
            assert!(
                (vitals.intelligence - intrinsic.intelligence - progress.absorbed_intelligence)
                    .abs()
                    < 0.001
            );
            assert!(
                (vitals.max_health - intrinsic.max_health - progress.absorbed_strength * 14.).abs()
                    < 0.01
            );
            assert!((vitals.max_health - vitals.health - 321.).abs() < 0.01);
            assert!((build.attack_damage - vitals.attack_damage() - 73.5).abs() < 0.01);
            assert_eq!(build.skills, skills);
            assert_eq!(vitals.kind, kind);
        }
    }

    #[test]
    fn absorbed_intrinsic_attributes_survive_growth_without_copying_existing_buffs() {
        let mut progress = RecruitmentProgress::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::for_hero(HeroKind::StormSavant, 10);
        vitals.strength += 50.; // Independent equipment bonus, not part of absorbed intrinsic stats.
        let intrinsic = HeroVitals::for_hero(vitals.kind, 10);
        absorb_and_recruit(
            HeroKind::StoneWarden,
            10,
            &mut progress,
            &mut build,
            &mut vitals,
        );
        assert_eq!(progress.absorbed_strength, intrinsic.strength);
        vitals.synchronize_level(HeroKind::StoneWarden, 11);
        let next = HeroVitals::for_hero(HeroKind::StoneWarden, 11);
        assert!((vitals.strength - next.strength - intrinsic.strength - 50.).abs() < 0.001);
        let before = vitals.strength;
        vitals.synchronize_level(HeroKind::StoneWarden, 11);
        assert_eq!(vitals.strength, before);
        for seed in 1..20 {
            let offers =
                roll_recruitment_offers(seed, 1, HeroKind::StormSavant, &HeroFusion::default());
            assert_eq!(offers.len(), 6);
            assert!(
                offers
                    .iter()
                    .all(|offer| offer.kind != HeroKind::StormSavant)
            );
            assert_eq!(
                offers
                    .iter()
                    .map(|offer| offer.kind)
                    .collect::<HashSet<_>>()
                    .len(),
                6
            );
        }
    }

    #[test]
    fn card_growth_survives_level_sync_without_healing_on_idle_frames() {
        use super::super::bond_catalog::BondCardId;
        use super::super::bonds::{BondRuntime, BondsPlugin};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<SelectedHero>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<PlayerBuild>()
            .add_plugins(BondsPlugin)
            .add_systems(Update, synchronize_hero_growth);
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards[BondCardId::JourneyStrength.index()] = true;
        app.update();
        let before = app.world().resource::<HeroVitals>().strength;
        app.world_mut().resource_mut::<HeroVitals>().health = 7.;
        for _ in 0..10 {
            app.update();
        }
        assert!((app.world().resource::<HeroVitals>().health - 7.).abs() < 0.001);
        let old_max = app.world().resource::<HeroVitals>().max_health;
        app.world_mut().resource_mut::<GameSession>().level = 2;
        app.update();
        let vitals = app.world().resource::<HeroVitals>();
        let base = HeroVitals::for_hero(vitals.kind, 2);
        assert!((vitals.strength - base.strength - 20.).abs() < 0.001);
        assert!(vitals.strength > before);
        assert!(
            (vitals.health - (7. + vitals.max_health - old_max)).abs() < 0.001,
            "level growth must not restore the old card health bonus"
        );
        assert!(app.world().contains_resource::<BondRuntime>());
        {
            let mut vitals = app.world_mut().resource_mut::<HeroVitals>();
            vitals.alive = false;
            vitals.health = 0.;
            vitals.deaths = 2;
            vitals.revive_remaining = 6.;
        }
        app.world_mut().resource_mut::<SelectedHero>().kind = HeroKind::StoneWarden;
        app.update();
        let dead = app.world().resource::<HeroVitals>();
        assert!(!dead.alive);
        assert_eq!(dead.health, 0.);
        assert_eq!(dead.deaths, 2);
        assert_eq!(dead.revive_remaining, 6.);
    }

    #[test]
    fn selector_status_panel_hides_only_after_hero_is_locked() {
        let mut app = App::new();
        app.init_resource::<SelectedHero>()
            .add_systems(Update, update_hero_selector_panel_visibility);
        let panel = app
            .world_mut()
            .spawn((HeroSelectorStatusPanel, Visibility::Inherited))
            .id();

        app.update();
        assert_eq!(
            app.world().get::<Visibility>(panel),
            Some(&Visibility::Inherited)
        );

        app.world_mut().resource_mut::<SelectedHero>().locked = true;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(panel),
            Some(&Visibility::Hidden)
        );
    }

    #[test]
    fn hero_offers_are_seeded_and_replayable() {
        let first = roll_hero_offers(0xA11CE, 0);
        let replay = roll_hero_offers(0xA11CE, 0);
        let another_seed = roll_hero_offers(0xB0B, 0);
        let next_generation = roll_hero_offers(0xA11CE, 1);

        assert_eq!(first, replay);
        assert_ne!(first, another_seed);
        assert_ne!(first, next_generation);
    }

    #[test]
    fn held_pointer_on_new_hero_card_does_not_commit() {
        let mut app = recruitment_app(10);
        app.update();
        let root = app.world().resource::<HeroState>().root.unwrap();
        let offer = app.world().resource::<HeroState>().preview.unwrap();
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Left);
        mouse.clear();
        app.insert_resource(mouse);
        app.world_mut()
            .spawn((HeroChoiceButton(offer), Interaction::Pressed));
        app.update();
        assert_eq!(app.world().resource::<HeroState>().root, Some(root));
        assert_eq!(
            app.world().resource::<SelectedHero>().kind,
            HeroKind::StormSavant
        );
        assert!(
            app.world()
                .resource::<RecruitmentProgress>()
                .pending
                .is_some()
        );
    }
    #[test]
    fn card_click_immediately_commits_initial_hero() {
        let mut app = App::new();
        app.init_resource::<HeroState>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<HeroFusion>()
            .init_resource::<SelectedHero>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .add_systems(Update, handle_hero_choice);
        let root = app.world_mut().spawn_empty().id();
        app.world_mut().resource_mut::<HeroState>().root = Some(root);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let offer = roll_hero_offers(0xA11CE, 0)[0];
        app.world_mut()
            .spawn((HeroChoiceButton(offer), Interaction::Pressed));
        app.update();
        let selected = app.world().resource::<SelectedHero>();
        assert!(selected.locked);
        assert_eq!(selected.kind, offer.kind);
        assert_eq!(app.world().resource::<HeroVitals>().kind, offer.kind);
        assert!(!app.world().resource::<GameSession>().paused);
        assert!(app.world().get_entity(root).is_err());
        let skill_index = SKILLS
            .iter()
            .position(|skill| skill.id == offer.starting_skill)
            .unwrap();
        assert_eq!(
            app.world().resource::<PlayerBuild>().skills[skill_index],
            HEROES[offer.kind.index()].rarity.starting_skill_rank()
        );
    }

    #[test]
    fn hero_rarity_weight_boundaries_are_exact() {
        let intelligence_tiers = [
            HeroKind::StormSavant,
            HeroKind::TideWalker,
            HeroKind::FrostOracle,
            HeroKind::SpiritKeeper,
        ];
        let expected = [
            (0, 0),
            (51, 0),
            (52, 1),
            (79, 1),
            (80, 2),
            (94, 2),
            (95, 3),
            (99, 3),
        ];
        for (roll, index) in expected {
            assert_eq!(weighted_hero_index(&intelligence_tiers, roll), index);
        }
    }

    #[test]
    fn hero_offers_are_unique_cover_all_attributes_and_use_valid_theme_skills() {
        for seed in [1, 2, 0xA11CE, 0xDEAD_BEEF] {
            let offers = roll_hero_offers(seed, 0);
            let unique: HashSet<HeroKind> = offers.iter().map(|offer| offer.kind).collect();
            assert_eq!(unique.len(), 3);
            assert!(
                offers
                    .iter()
                    .all(|offer| HEROES[offer.kind.index()].rarity == Rarity::R)
            );

            for primary in [
                Attribute::Strength,
                Attribute::Agility,
                Attribute::Intelligence,
            ] {
                assert!(
                    offers
                        .iter()
                        .any(|offer| { HEROES[offer.kind.index()].primary == primary })
                );
            }
            for offer in offers {
                let hero = HEROES[offer.kind.index()];
                assert!(hero.starting_skill_pool.contains(&offer.starting_skill));
                assert!(SKILLS.iter().any(|skill| skill.id == offer.starting_skill));
            }
        }
    }

    #[test]
    fn choosing_an_offer_grants_only_its_seeded_starting_skill() {
        for offer in roll_hero_offers(0x5EED, 0) {
            let mut build = PlayerBuild::default();
            build.skills.fill(3);
            let mut vitals = HeroVitals::default();
            apply_hero_stats(offer.kind, offer.starting_skill, 1, &mut build, &mut vitals);

            let granted: Vec<usize> = build
                .skills
                .iter()
                .enumerate()
                .filter_map(|(index, rank)| (*rank > 0).then_some(index))
                .collect();
            assert_eq!(granted.len(), 1);
            assert_eq!(SKILLS[granted[0]].id, offer.starting_skill);
            assert_eq!(
                build.skills[granted[0]],
                HEROES[offer.kind.index()].rarity.starting_skill_rank()
            );
        }
    }
}

#[derive(Component)]
enum HeroStaticCopy {
    Choose,
}
fn refresh_hero_language(
    language: Res<GameLanguage>,
    mut labels: Query<(&HeroStaticCopy, &mut Text)>,
) {
    if !language.is_changed() {
        return;
    }
    for (_, mut text) in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(language.pick("CHOOSE HERO", "选择英雄").into());
    }
}

#[cfg(test)]
mod live_language_tests {
    use super::*;

    #[test]
    fn language_switch_keeps_existing_recruits_and_preview() {
        let mut app = App::new();
        app.init_resource::<GameLanguage>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .init_resource::<HeroState>()
            .init_resource::<SelectedHero>()
            .init_resource::<draft::DraftArt>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<HeroFusion>()
            .init_resource::<GameSession>()
            .init_resource::<RunRng>()
            .add_systems(Update, (draft::refresh_language, draft::update_feedback));
        let offers = roll_hero_offers(0xA11CE, 2);
        let root = {
            let mut commands = app.world_mut().commands();
            spawn_hero_selector(
                &mut commands,
                &offers,
                &draft::DraftArt::default(),
                GameLanguage::English,
            )
        };
        app.world_mut().flush();
        {
            let mut state = app.world_mut().resource_mut::<HeroState>();
            state.root = Some(root);
            state.preview = Some(offers[1]);
            state.offer_generation = 2;
        }
        app.world_mut().resource_mut::<GameSession>().paused = true;
        let rng = app.world().resource::<RunRng>().0;
        for language in [GameLanguage::SimplifiedChinese, GameLanguage::English] {
            *app.world_mut().resource_mut::<GameLanguage>() = language;
            app.update();
            let state = app.world().resource::<HeroState>();
            assert_eq!(state.root, Some(root));
            assert_eq!(state.preview, Some(offers[1]));
            assert_eq!(state.offer_generation, 2);
            assert!(app.world().resource::<GameSession>().paused);
            assert_eq!(app.world().resource::<RunRng>().0, rng);
            let choices: Vec<_> = app
                .world_mut()
                .query::<&HeroChoiceButton>()
                .iter(app.world())
                .map(|button| button.0)
                .collect();
            assert_eq!(choices.len(), offers.len());
            assert!(offers.iter().all(|offer| choices.contains(offer)));
            let headings: Vec<_> = app
                .world_mut()
                .query_filtered::<&Text, With<draft::DraftHeading>>()
                .iter(app.world())
                .map(|text| text.0.clone())
                .collect();
            assert_eq!(
                headings,
                [language.pick("Click a basic commander to begin", "点击基础指挥官开始远征")]
            );
            let names: Vec<_> = app
                .world_mut()
                .query_filtered::<&Text, With<draft::DraftCardName>>()
                .iter(app.world())
                .map(|text| text.0.clone())
                .collect();
            assert!(
                offers
                    .iter()
                    .all(|offer| names.contains(&hero_name(language, offer.kind)))
            );
        }
    }
}

pub(super) fn initial_offer_kinds(seed: u64) -> Vec<HeroKind> {
    roll_hero_offers(seed, 0)
        .into_iter()
        .map(|offer| offer.kind)
        .collect()
}
