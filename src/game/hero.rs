use bevy::prelude::*;

use super::combat::SurvivalSet;
use super::content::{Attribute, HEROES, SKILLS};
use super::{
    Autoplay, GameAssets, GameSession, Hero, HeroIdentity, HeroKind, HeroProgressionDelta,
    HeroVisual, HeroVitals, PlayerBuild, SelectedHero,
};

pub struct HeroPlugin;

impl Plugin for HeroPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeroState>()
            .add_systems(Startup, setup_hero_ui)
            .add_systems(
                Update,
                (
                    open_hero_selector,
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

#[derive(Resource, Default)]
struct HeroState {
    selected: u8,
    locked: bool,
    evolution_stage: u8,
    root: Option<Entity>,
}

#[derive(Component)]
struct HeroSelectButton;

#[derive(Component)]
struct HeroSelectorStatusPanel;

#[derive(Component)]
struct HeroChoiceButton(u8);

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
                left: px(22),
                top: px(212),
                width: px(205),
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
                        Text::new("F1  CHOOSE HERO"),
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
    keyboard: Res<ButtonInput<KeyCode>>,
    interactions: Query<&Interaction, (With<HeroSelectButton>, Changed<Interaction>)>,
    selected_hero: Res<SelectedHero>,
    mut session: ResMut<GameSession>,
    mut state: ResMut<HeroState>,
) {
    if selected_hero.locked
        || state.locked
        || state.root.is_some()
        || session.paused
        || session.finished
    {
        return;
    }
    let requested = keyboard.just_pressed(KeyCode::F1)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    if !requested {
        return;
    }
    session.paused = true;
    state.root = Some(spawn_hero_selector(&mut commands));
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

fn spawn_hero_selector(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(16),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.04, 0.92)),
            GlobalZIndex(70),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("CHOOSE YOUR COMMANDER"),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::srgb(0.76, 0.94, 1.0)),
            ));
            root.spawn(Node {
                width: percent(82),
                max_width: px(940),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                row_gap: px(12),
                column_gap: px(12),
                ..default()
            })
            .with_children(|grid| {
                for (index, hero) in HEROES.iter().enumerate() {
                    let primary = match hero.primary {
                        Attribute::Strength => "STRENGTH",
                        Attribute::Agility => "AGILITY",
                        Attribute::Intelligence => "INTELLIGENCE",
                    };
                    grid.spawn((
                        Button,
                        HeroChoiceButton(index as u8),
                        Node {
                            width: percent(30),
                            min_width: px(245),
                            height: px(125),
                            padding: UiRect::all(px(13)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            row_gap: px(8),
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(12)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.055, 0.11, 0.18)),
                        BorderColor::all(hero_color(index)),
                    ))
                    .with_children(|card| {
                        card.spawn((
                            Text::new(display_id(hero.id)),
                            TextFont {
                                font_size: FontSize::Px(20.0),
                                ..default()
                            },
                            TextColor(Color::WHITE),
                        ));
                        card.spawn((
                            Text::new(format!(
                                "{primary}\nSTART: {}",
                                display_id(hero.starting_skill)
                            )),
                            TextFont {
                                font_size: FontSize::Px(13.0),
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
    for (interaction, choice) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        state.selected = choice.0.min((HEROES.len() - 1) as u8);
        state.locked = true;
        selected_hero.kind = HeroKind::from_index(state.selected as usize);
        selected_hero.locked = true;
        for mut identity in &mut identities {
            identity.0 = selected_hero.kind;
        }
        apply_hero_stats(state.selected, session.level, &mut build, &mut vitals);
        if let Some(assets) = assets.as_deref() {
            for mut scene in &mut visuals {
                scene.0 = assets.hero_scenes[state.selected as usize].clone();
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
    vitals: Res<HeroVitals>,
    mut hero: Query<&mut Transform, With<Hero>>,
) {
    const LEVELS: [u32; 3] = [3, 9, 15];
    if state.evolution_stage >= 3
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

fn spawn_evolution(commands: &mut Commands, hero_index: u8, stage: u8) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(22),
                ..default()
            },
            BackgroundColor(Color::srgba(0.015, 0.02, 0.035, 0.92)),
            GlobalZIndex(75),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new(format!(
                    "{} ASCENSION {}",
                    display_id(HEROES[hero_index as usize].id),
                    stage + 1
                )),
                TextFont {
                    font_size: FontSize::Px(32.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.78, 0.28)),
            ));
            root.spawn(Node {
                width: percent(72),
                max_width: px(820),
                height: px(230),
                flex_direction: FlexDirection::Row,
                column_gap: px(18),
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
                width: percent(47),
                min_width: px(280),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(16),
                border: UiRect::all(px(3)),
                border_radius: BorderRadius::all(px(15)),
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
                    font_size: FontSize::Px(25.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            card.spawn((
                Text::new(description),
                TextFont {
                    font_size: FontSize::Px(17.0),
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
        apply_hero_stats(
            selected.kind.index() as u8,
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

fn apply_hero_stats(index: u8, level: u32, build: &mut PlayerBuild, vitals: &mut HeroVitals) {
    let hero = HEROES[index as usize];
    vitals.reset_for_hero(HeroKind::from_index(index as usize), level);
    build.attack_damage = vitals.attack_damage();
    build.attacks_per_second = 1.45 + vitals.agility * 0.028;
    build.move_speed = 5.4 + vitals.agility * 0.035;
    build.projectile_speed = 17.0 + vitals.intelligence * 0.08;
    build.attack_range = 8.5 + vitals.intelligence * 0.025;
    build.skills.fill(0);
    if let Some(skill_index) = SKILLS
        .iter()
        .position(|skill| skill.id == hero.starting_skill)
    {
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
            display_id(HEROES[state.selected as usize].id),
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
    ][index]
}

#[cfg(test)]
mod tests {
    use super::*;

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
