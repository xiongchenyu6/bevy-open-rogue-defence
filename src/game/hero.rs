use bevy::{prelude::*, window::PrimaryWindow};

use super::combat::SurvivalSet;
use super::content::{Attribute, ContentId, HEROES, SKILLS};
use super::mobile_layout::hud_metrics;
use super::{
    Autoplay, GameAssets, GameSession, Hero, HeroIdentity, HeroKind, HeroProgressionDelta,
    HeroVisual, HeroVitals, PlayerBuild, RunRng, RunSeed, SelectedHero,
};

pub struct HeroPlugin;

const OVERLAY_PADDING: f32 = 8.0;
const HERO_HEADING_SIZE: f32 = 22.0;
const HERO_GRID_WIDTH_PERCENT: f32 = 96.0;
const HERO_GRID_MAX_WIDTH: f32 = 1_000.0;
const HERO_GRID_HEIGHT: f32 = 292.0;
const HERO_CARD_WIDTH_PERCENT: f32 = 31.0;
const HERO_CARD_HEIGHT_PERCENT: f32 = 48.0;
const HERO_GRID_GAP: f32 = 8.0;
const EVOLUTION_GRID_WIDTH_PERCENT: f32 = 94.0;
const EVOLUTION_GRID_MAX_WIDTH: f32 = 900.0;
const EVOLUTION_GRID_HEIGHT: f32 = 260.0;
const EVOLUTION_CARD_WIDTH_PERCENT: f32 = 48.0;
const EVOLUTION_GRID_GAP: f32 = 10.0;
const MIN_TOUCH_TARGET: f32 = 48.0;
const HERO_OFFER_COUNT: usize = 6;
const FREE_HERO_REROLLS: u8 = 1;
const HERO_OFFER_SEED_SALT: u64 = 0x4845_524F_5F4F_4646;
const _: () = assert!(MIN_TOUCH_TARGET >= 48.0);

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroState>()
            .add_systems(Startup, setup_hero_ui)
            .add_systems(
                Update,
                (
                    open_hero_selector,
                    handle_hero_reroll,
                    update_hero_selector_safe_area,
                    handle_hero_choice,
                    update_hero_selector_panel_visibility,
                    synchronize_hero_growth,
                    trigger_evolution,
                    handle_evolution_choice,
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
    locked: bool,
    evolution_stage: u8,
    root: Option<Entity>,
    offer_generation: u32,
    rerolls_remaining: u8,
    ignore_choice_this_frame: bool,
}

impl Default for HeroState {
    fn default() -> Self {
        Self {
            selected: HeroKind::StormSavant,
            locked: false,
            evolution_stage: 0,
            root: None,
            offer_generation: 0,
            rerolls_remaining: FREE_HERO_REROLLS,
            ignore_choice_this_frame: false,
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
struct HeroChoiceButton(HeroOffer);

#[derive(Component)]
struct HeroRerollButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HeroOffer {
    kind: HeroKind,
    starting_skill: ContentId,
}

#[derive(Component)]
struct EvolutionChoiceButton(u8);

#[derive(Component)]
struct HeroStatusText;

fn setup_hero_ui(mut commands: Commands) {
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
                Text::new("STORM SAVANT  [R]"),
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
                        Text::new("CHOOSE HERO"),
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
    windows: Query<&Window, With<PrimaryWindow>>,
    run_seed: Res<RunSeed>,
    selected_hero: Res<SelectedHero>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<HeroState>,
) {
    if autoplay.is_some()
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
    state.root = Some(spawn_hero_selector(
        &mut commands,
        &offers,
        state.rerolls_remaining > 0,
        hero_selector_safe_x(&windows),
    ));
}

fn handle_hero_reroll(
    mut commands: Commands,
    interactions: Query<&Interaction, (With<HeroRerollButton>, Changed<Interaction>)>,
    windows: Query<&Window, With<PrimaryWindow>>,
    run_seed: Res<RunSeed>,
    mut state: ResMut<HeroState>,
) {
    let requested = interactions
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed);
    if !requested || state.rerolls_remaining == 0 {
        return;
    }
    let Some(root) = state.root.take() else {
        return;
    };

    commands.entity(root).despawn();
    state.offer_generation = state.offer_generation.saturating_add(1);
    state.rerolls_remaining -= 1;
    state.ignore_choice_this_frame = true;
    let offers = roll_hero_offers(run_seed.0, state.offer_generation);
    state.root = Some(spawn_hero_selector(
        &mut commands,
        &offers,
        false,
        hero_selector_safe_x(&windows),
    ));
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

fn update_hero_selector_safe_area(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(
        &mut Node,
        Option<&HeroSelectorRoot>,
        Option<&HeroRerollButton>,
    )>,
) {
    let safe_x = hero_selector_safe_x(&windows);
    for (mut node, root, reroll) in &mut nodes {
        if root.is_some() {
            node.padding.left = px(safe_x);
            node.padding.right = px(safe_x);
        }
        if reroll.is_some() {
            node.right = px(safe_x);
        }
    }
}

fn spawn_hero_selector(
    commands: &mut Commands,
    offers: &[HeroOffer; HERO_OFFER_COUNT],
    can_reroll: bool,
    safe_x: f32,
) -> Entity {
    commands
        .spawn((
            HeroSelectorRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                padding: UiRect {
                    left: px(safe_x),
                    right: px(safe_x),
                    top: px(OVERLAY_PADDING),
                    bottom: px(OVERLAY_PADDING),
                },
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(HERO_GRID_GAP),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.04, 0.985)),
            GlobalZIndex(70),
        ))
        .with_children(|root| {
            if can_reroll {
                root.spawn((
                    Button,
                    HeroRerollButton,
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(safe_x),
                        top: px(8),
                        width: px(108),
                        height: px(MIN_TOUCH_TARGET),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(8)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.12, 0.26, 0.38)),
                    BorderColor::all(Color::srgb(0.28, 0.72, 0.92)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("REROLL FREE"),
                        TextFont {
                            font_size: FontSize::Px(11.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });
            }
            root.spawn((
                Text::new("COMMANDER DRAFT  |  6 OF 12"),
                TextFont {
                    font_size: FontSize::Px(HERO_HEADING_SIZE),
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.94, 1.0)),
            ));
            root.spawn(Node {
                width: percent(HERO_GRID_WIDTH_PERCENT),
                max_width: px(HERO_GRID_MAX_WIDTH),
                height: px(HERO_GRID_HEIGHT),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Center,
                row_gap: px(HERO_GRID_GAP),
                column_gap: px(HERO_GRID_GAP),
                ..default()
            })
            .with_children(|grid| {
                for offer in offers.iter().copied() {
                    let hero = HEROES[offer.kind.index()];
                    let primary = match hero.primary {
                        Attribute::Strength => "STRENGTH",
                        Attribute::Agility => "AGILITY",
                        Attribute::Intelligence => "INTELLIGENCE",
                    };
                    grid.spawn((
                        Button,
                        HeroChoiceButton(offer),
                        Node {
                            width: percent(HERO_CARD_WIDTH_PERCENT),
                            min_width: px(MIN_TOUCH_TARGET),
                            height: percent(HERO_CARD_HEIGHT_PERCENT),
                            min_height: px(MIN_TOUCH_TARGET),
                            padding: UiRect::all(px(6)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: px(5),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.055, 0.11, 0.18)),
                        BorderColor::all(hero_color(offer.kind.index())),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(display_id(hero.id)),
                            TextFont {
                                font_size: FontSize::Px(17.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                            TextLayout::justify(Justify::Center),
                        ));
                        card.spawn((
                            Text::new(format!(
                                "{primary}\nSTART: {}",
                                display_id(offer.starting_skill)
                            )),
                            TextFont {
                                font_size: FontSize::Px(11.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.68, 0.78, 0.88)),
                            TextLayout::justify(Justify::Center),
                        ));
                    });
                }
            });
        })
        .id()
}

fn handle_hero_choice(
    mut commands: Commands,
    interactions: Query<(&Interaction, &HeroChoiceButton), Changed<Interaction>>,
    assets: Option<Res<GameAssets>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
    mut state: ResMut<HeroState>,
    mut selected_hero: ResMut<SelectedHero>,
    mut identities: Query<&mut HeroIdentity, With<Hero>>,
    mut visuals: Query<&mut WorldAssetRoot, With<HeroVisual>>,
) {
    if std::mem::take(&mut state.ignore_choice_this_frame) {
        return;
    }
    for (interaction, choice) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.selected = choice.0.kind;
        state.locked = true;
        selected_hero.kind = state.selected;
        selected_hero.locked = true;
        for mut identity in &mut identities {
            identity.0 = selected_hero.kind;
        }
        apply_hero_stats(
            state.selected,
            choice.0.starting_skill,
            session.level,
            &mut build,
            &mut vitals,
        );
        if let Some(assets) = assets.as_deref() {
            for mut scene in &mut visuals {
                scene.0 = assets.hero_scenes[state.selected.index()].clone();
            }
        }
        close_overlay(&mut commands, &mut session, &mut state);
        break;
    }
}

fn trigger_evolution(
    mut commands: Commands,
    autoplay: Option<Res<Autoplay>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut state: ResMut<HeroState>,
    selected_hero: Res<SelectedHero>,
    vitals: Res<HeroVitals>,
    mut hero: Query<&mut Transform, With<Hero>>,
) {
    const LEVELS: [u32; 3] = [3, 9, 15];
    if !selected_hero.locked
        || state.evolution_stage >= 3
        || state.root.is_some()
        || session.paused
        || session.finished
        || !vitals.alive
        || vitals.health <= 0.0
        || session.level < LEVELS[state.evolution_stage as usize]
    {
        return;
    }
    if autoplay.is_some() {
        let branch = state.evolution_stage % 2;
        apply_evolution(branch, &mut build, &mut session);
        state.evolution_stage += 1;
        if let Ok(mut transform) = hero.single_mut() {
            transform.scale = Vec3::splat(1.0 + state.evolution_stage as f32 * 0.07);
        }
        return;
    }
    session.paused = true;
    state.root = Some(spawn_evolution(
        &mut commands,
        state.selected,
        state.evolution_stage,
    ));
}

fn spawn_evolution(commands: &mut Commands, hero_kind: HeroKind, stage: u8) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                padding: UiRect::all(px(OVERLAY_PADDING)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(EVOLUTION_GRID_GAP),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.02, 0.035, 0.92)),
            GlobalZIndex(75),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!(
                    "{} ASCENSION {}",
                    display_id(HEROES[hero_kind.index()].id),
                    stage + 1
                )),
                TextFont {
                    font_size: FontSize::Px(HERO_HEADING_SIZE),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.28)),
            ));
            root.spawn(Node {
                width: percent(EVOLUTION_GRID_WIDTH_PERCENT),
                max_width: px(EVOLUTION_GRID_MAX_WIDTH),
                height: px(EVOLUTION_GRID_HEIGHT),
                flex_direction: FlexDirection::Row,
                column_gap: px(EVOLUTION_GRID_GAP),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|row| {
                spawn_evolution_card(
                    row,
                    0,
                    "WAR PATH",
                    "+28% damage\n+12% attack speed\nstronger projectiles",
                );
                spawn_evolution_card(
                    row,
                    1,
                    "MYSTIC PATH",
                    "+1 multishot\n+10% move speed\nrepair the gate",
                );
            });
        })
        .id()
}

fn spawn_evolution_card(
    parent: &mut ChildSpawnerCommands,
    branch: u8,
    title: &'static str,
    description: &'static str,
) {
    parent
        .spawn((
            Button,
            EvolutionChoiceButton(branch),
            Node {
                width: percent(EVOLUTION_CARD_WIDTH_PERCENT),
                min_width: px(MIN_TOUCH_TARGET),
                min_height: px(MIN_TOUCH_TARGET),
                padding: UiRect::all(px(8)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(10),
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.075, 0.10, 0.17)),
            BorderColor::all(if branch == 0 {
                Color::srgb(1.0, 0.34, 0.16)
            } else {
                Color::srgb(0.25, 0.78, 1.0)
            }),
        ))
        .with_children(|card| {
            card.spawn((
                Text::new(title),
                TextFont {
                    font_size: FontSize::Px(20.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            card.spawn((
                Text::new(description),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.72, 0.8, 0.9)),
                TextLayout::justify(Justify::Center),
            ));
        });
}

fn handle_evolution_choice(
    mut commands: Commands,
    interactions: Query<(&Interaction, &EvolutionChoiceButton), Changed<Interaction>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut state: ResMut<HeroState>,
    mut hero: Query<&mut Transform, With<Hero>>,
) {
    for (interaction, choice) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        apply_evolution(choice.0, &mut build, &mut session);
        state.evolution_stage += 1;
        if let Ok(mut transform) = hero.single_mut() {
            transform.scale = Vec3::splat(1.0 + state.evolution_stage as f32 * 0.07);
        }
        close_overlay(&mut commands, &mut session, &mut state);
        break;
    }
}

fn synchronize_hero_growth(
    selected: Res<SelectedHero>,
    session: Res<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
) {
    if vitals.kind != selected.kind {
        let starting_skill = HEROES[selected.kind.index()].starting_skill;
        apply_hero_stats(
            selected.kind,
            starting_skill,
            session.level,
            &mut build,
            &mut vitals,
        );
        return;
    }
    let delta = vitals.synchronize_level(selected.kind, session.level);
    apply_growth_delta(&mut build, delta);
}

fn apply_growth_delta(build: &mut PlayerBuild, delta: HeroProgressionDelta) {
    build.attack_damage += delta.primary;
    build.attacks_per_second += delta.agility * 0.028;
    build.move_speed += delta.agility * 0.035;
    build.projectile_speed += delta.intelligence * 0.08;
    build.attack_range += delta.intelligence * 0.025;
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
    build.attacks_per_second = 1.45 + vitals.agility * 0.028;
    build.move_speed = 5.4 + vitals.agility * 0.035;
    build.projectile_speed = 17.0 + vitals.intelligence * 0.08;
    build.attack_range = 8.5 + vitals.intelligence * 0.025;
    build.skills.fill(0);
    if let Some(skill_index) = SKILLS.iter().position(|skill| skill.id == starting_skill) {
        build.skills[skill_index] = 1;
    }
}

fn apply_evolution(branch: u8, build: &mut PlayerBuild, session: &mut GameSession) {
    if branch == 0 {
        build.attack_damage *= 1.28;
        build.attacks_per_second *= 1.12;
        build.projectile_speed *= 1.08;
    } else {
        build.multishot = build.multishot.saturating_add(1).min(9);
        build.move_speed *= 1.10;
        session.core_max_hp += 120.0;
        session.core_hp = (session.core_hp + 180.0).min(session.core_max_hp);
    }
}

fn update_hero_status(state: Res<HeroState>, mut text: Query<&mut Text, With<HeroStatusText>>) {
    if let Ok(mut text) = text.single_mut() {
        let rarity = ["R", "SR", "SSR", "UR"][state.evolution_stage as usize];
        **text = format!(
            "{}  [{}]",
            display_id(HEROES[state.selected.index()].id),
            rarity
        );
    }
}

fn close_overlay(commands: &mut Commands, session: &mut GameSession, state: &mut HeroState) {
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    session.paused = false;
}

fn roll_hero_offers(seed: u64, generation: u32) -> [HeroOffer; HERO_OFFER_COUNT] {
    let generation_seed = u64::from(generation)
        .wrapping_add(1)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    let mut rng = RunRng::from_seed(seed ^ HERO_OFFER_SEED_SALT ^ generation_seed);
    let mut selected = Vec::with_capacity(HERO_OFFER_COUNT);

    for primary in [
        Attribute::Strength,
        Attribute::Agility,
        Attribute::Intelligence,
    ] {
        let mut candidates: Vec<HeroKind> = HeroKind::ALL
            .into_iter()
            .filter(|kind| HEROES[kind.index()].primary == primary)
            .collect();
        selected.push(take_random_hero(&mut candidates, &mut rng));
    }

    let mut remaining: Vec<HeroKind> = HeroKind::ALL
        .into_iter()
        .filter(|kind| !selected.contains(kind))
        .collect();
    while selected.len() < HERO_OFFER_COUNT {
        selected.push(take_random_hero(&mut remaining, &mut rng));
    }
    for upper in (1..selected.len()).rev() {
        let swap_with = rng.next_u32() as usize % (upper + 1);
        selected.swap(upper, swap_with);
    }

    std::array::from_fn(|index| {
        let kind = selected[index];
        let skill_pool = HEROES[kind.index()].starting_skill_pool;
        HeroOffer {
            kind,
            starting_skill: skill_pool[rng.next_u32() as usize % skill_pool.len()],
        }
    })
}

fn hero_selector_safe_x(windows: &Query<&Window, With<PrimaryWindow>>) -> f32 {
    windows
        .single()
        .map(|window| hud_metrics(window.width(), window.height()).safe_x)
        .unwrap_or(OVERLAY_PADDING)
}

fn take_random_hero(pool: &mut Vec<HeroKind>, rng: &mut RunRng) -> HeroKind {
    let index = rng.next_u32() as usize % pool.len();
    pool.swap_remove(index)
}

fn display_id(id: &str) -> String {
    id.replace('_', " ").to_ascii_uppercase()
}

fn hero_color(index: usize) -> Color {
    [
        Color::srgb(0.18, 0.75, 1.0),
        Color::srgb(1.0, 0.35, 0.12),
        Color::srgb(0.72, 0.76, 0.82),
        Color::srgb(0.62, 0.32, 1.0),
        Color::srgb(0.12, 0.82, 0.68),
        Color::srgb(1.0, 0.75, 0.18),
        Color::srgb(0.48, 0.82, 0.94),
        Color::srgb(0.72, 0.18, 0.10),
        Color::srgb(0.78, 0.24, 0.94),
        Color::srgb(0.28, 0.76, 0.32),
        Color::srgb(1.0, 0.68, 0.14),
        Color::srgb(0.48, 0.16, 0.72),
    ][index]
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    fn overlay_width(width: f32, percent: f32, max_width: f32) -> f32 {
        ((width - OVERLAY_PADDING * 2.0) * percent / 100.0).min(max_width)
    }

    #[test]
    fn hero_and_evolution_overlays_fit_compact_landscape() {
        for (width, height) in [(667.0, 375.0), (844.0, 390.0), (1280.0, 720.0)] {
            let safe_x = hud_metrics(width, height).safe_x;
            let hero_grid_width = overlay_width(
                width - safe_x * 2.0 + OVERLAY_PADDING * 2.0,
                HERO_GRID_WIDTH_PERCENT,
                HERO_GRID_MAX_WIDTH,
            );
            let hero_row_width =
                hero_grid_width * HERO_CARD_WIDTH_PERCENT / 100.0 * 3.0 + HERO_GRID_GAP * 2.0;
            let hero_grid_rows =
                HERO_GRID_HEIGHT * HERO_CARD_HEIGHT_PERCENT / 100.0 * 2.0 + HERO_GRID_GAP;
            let hero_vertical =
                OVERLAY_PADDING * 2.0 + HERO_HEADING_SIZE * 1.2 + HERO_GRID_GAP + HERO_GRID_HEIGHT;
            assert!(
                hero_row_width <= hero_grid_width,
                "hero row at {width}x{height}"
            );
            assert!(
                hero_grid_rows <= HERO_GRID_HEIGHT,
                "hero rows at {width}x{height}"
            );
            assert!(hero_vertical <= height, "hero overlay at {width}x{height}");
            if width < 1_000.0 || height < 520.0 {
                assert!(safe_x >= 28.0, "hero safe inset at {width}x{height}");
            }

            let evolution_width = overlay_width(
                width,
                EVOLUTION_GRID_WIDTH_PERCENT,
                EVOLUTION_GRID_MAX_WIDTH,
            );
            let evolution_row =
                evolution_width * EVOLUTION_CARD_WIDTH_PERCENT / 100.0 * 2.0 + EVOLUTION_GRID_GAP;
            let evolution_vertical = OVERLAY_PADDING * 2.0
                + HERO_HEADING_SIZE * 1.2
                + EVOLUTION_GRID_GAP
                + EVOLUTION_GRID_HEIGHT;
            assert!(
                evolution_row <= evolution_width,
                "evolution row at {width}x{height}"
            );
            assert!(
                evolution_vertical <= height,
                "evolution overlay at {width}x{height}"
            );
        }
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
    fn hero_offers_are_seeded_replayable_and_reroll_once() {
        let first = roll_hero_offers(0xA11CE, 0);
        let replay = roll_hero_offers(0xA11CE, 0);
        let another_seed = roll_hero_offers(0xB0B, 0);
        let rerolled = roll_hero_offers(0xA11CE, 1);

        assert_eq!(first, replay);
        assert_ne!(first, another_seed);
        assert_ne!(first, rerolled);
        assert_eq!(HeroState::default().rerolls_remaining, 1);
    }

    #[test]
    fn hero_offers_are_unique_cover_all_attributes_and_use_valid_theme_skills() {
        for seed in [1, 2, 0xA11CE, 0xDEAD_BEEF] {
            let offers = roll_hero_offers(seed, 0);
            let unique: HashSet<HeroKind> = offers.iter().map(|offer| offer.kind).collect();
            assert_eq!(unique.len(), HERO_OFFER_COUNT);

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
            assert_eq!(build.skills[granted[0]], 1);
        }
    }

    #[test]
    fn level_growth_preserves_existing_evolution_and_skills() {
        let mut build = PlayerBuild::default();
        let mut session = GameSession::default();
        let mut vitals = HeroVitals::default();
        build.skills[5] = 3;
        apply_evolution(0, &mut build, &mut session);
        let evolved_attack = build.attack_damage;

        let delta = vitals.synchronize_level(HeroKind::StormSavant, 2);
        apply_growth_delta(&mut build, delta);

        assert!(build.attack_damage > evolved_attack);
        assert_eq!(build.skills[5], 3);
        assert!(build.projectile_speed > 19.24 * 1.08);
    }
}
