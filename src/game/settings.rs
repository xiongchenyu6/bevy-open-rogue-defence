//! Settings restores the prior pause and edits persisted player preferences.
mod audio_controls;
mod entry_layout;
mod run_actions;
pub use run_actions::SettingsDepartureInputSet;
#[cfg(not(target_arch = "wasm32"))]
mod display_controls;
use bevy::{prelude::*, ui::UiSystems, window::PrimaryWindow};

use super::{CaptureTarget, GameLanguage, GameSession};

const GOLD: Color = Color::srgb(0.90, 0.72, 0.42);
const IVORY: Color = Color::srgb(0.96, 0.93, 0.85);
const MUTED: Color = Color::srgb(0.60, 0.68, 0.69);
const PANEL: Color = Color::srgb(0.035, 0.058, 0.067);
const EDGE: Color = Color::srgb(0.24, 0.31, 0.32);

pub(super) struct SettingsPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettingsInputSet;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        audio_controls::install(app);
        run_actions::install(app);
        entry_layout::install(app);
        #[cfg(not(target_arch = "wasm32"))]
        app.add_plugins(display_controls::DisplayControlsPlugin);
        app.init_resource::<SettingsState>()
            .add_systems(Startup, setup_entry)
            .add_systems(
                PreUpdate,
                handle_settings_input
                    .after(UiSystems::Focus)
                    .in_set(SettingsInputSet),
            )
            .add_systems(
                Update,
                (
                    update_settings_copy,
                    update_settings_layout,
                    update_tutorial_entry,
                ),
            );
    }
}

#[derive(Resource, Default)]
pub(crate) struct SettingsState {
    pub(crate) open: bool,
    root: Option<Entity>,
    prior_paused: bool,
    save_failed: bool,
    confirmation: Option<Entity>,
}

impl SettingsState {
    fn begin(&mut self, session: &mut GameSession) {
        self.prior_paused = session.paused;
        self.open = true;
        self.save_failed = false;
        session.paused = true;
    }

    fn finish(&mut self, session: &mut GameSession) {
        self.open = false;
        session.paused = self.prior_paused;
    }
}

#[derive(Component)]
struct SettingsOpen;
#[derive(Component)]
struct SettingsClose;
#[derive(Component)]
struct SettingsTutorial;
#[derive(Component)]
struct SettingsPanel;
#[derive(Component)]
struct SettingsHeader;
#[derive(Component)]
struct SettingsLanguageRow;
#[derive(Component)]
struct SettingsLanguageButton(GameLanguage);
#[derive(Component)]
struct SettingsSelected(GameLanguage);
#[derive(Component)]
struct SettingsHeading;
#[derive(Component)]
struct SettingsCopy(CopyKind);

#[derive(Clone, Copy)]
enum CopyKind {
    Title,
    Section,
    Intro,
    Selected,
    SaveNotice,
    Back,
    Tutorial,
}

fn label(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn setup_entry(mut commands: Commands) {
    commands
        .spawn((
            Button,
            SettingsOpen,
            Name::new("Settings open"),
            Node {
                position_type: PositionType::Absolute,
                left: percent(50),
                top: px(8),
                margin: UiRect::left(px(-40)),
                width: px(80),
                height: px(44),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            GlobalZIndex(510),
            BackgroundColor(Color::srgba(0.025, 0.045, 0.052, 0.98)),
            BorderColor::all(GOLD),
        ))
        .with_children(|button| {
            button.spawn((SettingsCopy(CopyKind::Title), label("", 13.0, IVORY)));
        });
}

fn spawn_settings(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            Name::new("Settings page"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            bevy::ui::FocusPolicy::Block,
            GlobalZIndex(500),
            BackgroundColor(Color::srgb(0.014, 0.025, 0.032)),
        ))
        .with_children(|root| {
            root.spawn((
                SettingsPanel,
                Node {
                    flex_direction: FlexDirection::Column,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(PANEL),
                BorderColor::all(EDGE),
            ))
            .with_children(|panel| {
                panel
                    .spawn((
                        SettingsHeader,
                        Node {
                            flex_direction: FlexDirection::Column,
                            row_gap: px(6),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|header| {
                        header.spawn((
                            SettingsHeading,
                            SettingsCopy(CopyKind::Title),
                            label("", 32.0, IVORY),
                        ));
                        header.spawn((SettingsCopy(CopyKind::Intro), label("", 13.0, MUTED)));
                    });
                panel.spawn((SettingsCopy(CopyKind::Section), label("", 14.0, GOLD)));
                panel
                    .spawn((
                        SettingsLanguageRow,
                        Node {
                            width: percent(100),
                            column_gap: px(14),
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ))
                    .with_children(|row| {
                        for (language, native, hint, hook) in [
                            (GameLanguage::English, "English", "EN", "Settings English"),
                            (
                                GameLanguage::SimplifiedChinese,
                                "简体中文",
                                "中",
                                "Settings Chinese",
                            ),
                        ] {
                            row.spawn((
                                Button,
                                SettingsLanguageButton(language),
                                Name::new(hook),
                                Node {
                                    flex_basis: px(0),
                                    flex_grow: 1.0,
                                    min_width: px(0),
                                    min_height: px(72),
                                    padding: UiRect::all(px(10)),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::Center,
                                    row_gap: px(5),
                                    border: UiRect::all(px(2)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.06, 0.095, 0.105)),
                                BorderColor::all(EDGE),
                            ))
                            .with_children(|button| {
                                button.spawn(label(hint, 11.0, MUTED));
                                button.spawn(label(native, 20.0, IVORY));
                                button.spawn((
                                    SettingsSelected(language),
                                    SettingsCopy(CopyKind::Selected),
                                    label("", 11.0, GOLD),
                                ));
                            });
                        }
                    });
                super::selection_detail::spawn_toggle(panel);
                audio_controls::spawn(panel);
                panel.spawn((
                    SettingsCopy(CopyKind::SaveNotice),
                    label("", 12.0, MUTED),
                    Node {
                        flex_grow: 1.0,
                        ..default()
                    },
                ));
                panel
                    .spawn((
                        Button,
                        SettingsTutorial,
                        Name::new("Settings tutorial"),
                        Node {
                            min_height: px(42),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(PANEL),
                    ))
                    .with_child((SettingsCopy(CopyKind::Tutorial), label("", 14.0, IVORY)));
                panel
                    .spawn(Node {
                        width: percent(100),
                        column_gap: px(6),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Button,
                            SettingsClose,
                            Name::new("Settings close"),
                            Node {
                                flex_basis: px(0),
                                flex_grow: 1.0,
                                min_width: px(0),
                                height: px(48),
                                min_height: px(48),
                                flex_shrink: 0.0,
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(GOLD),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                SettingsCopy(CopyKind::Back),
                                label("", 13.0, Color::srgb(0.035, 0.055, 0.055)),
                            ));
                        });
                        run_actions::spawn(row);
                        #[cfg(not(target_arch = "wasm32"))]
                        display_controls::spawn(row);
                    });
            });
        })
        .id()
}

fn handle_settings_input(
    mut commands: Commands,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    open_buttons: Query<&Interaction, (With<SettingsOpen>, Changed<Interaction>)>,
    close_buttons: Query<&Interaction, (With<SettingsClose>, Changed<Interaction>)>,
    language_buttons: Query<(&Interaction, &SettingsLanguageButton), Changed<Interaction>>,
    mut state: ResMut<SettingsState>,
    mut tutorial: Option<ResMut<super::OnboardingState>>,
    tutorial_buttons: Query<&Interaction, (With<SettingsTutorial>, Changed<Interaction>)>,
    equipment: Option<Res<super::equipment_ui::EquipmentPanelState>>,
    mut session: ResMut<GameSession>,
    mut language: ResMut<GameLanguage>,
) {
    if !state.open {
        if tutorial.as_ref().is_some_and(|guide| guide.input_blocked) {
            return;
        }
        if equipment.is_some_and(|panel| panel.open()) {
            return;
        }
        if !open_buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
        {
            return;
        }
        state.begin(&mut session);
        state.root = Some(spawn_settings(&mut commands));
        keyboard.clear();
        return;
    }
    if state.confirmation.is_some() {
        keyboard.clear();
        return;
    }
    if tutorial_buttons.iter().any(|i| *i == Interaction::Pressed)
        && let Some(tutorial) = tutorial.as_deref_mut()
    {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.finish(&mut session);
        tutorial.replay_requested = true;
        keyboard.clear();
        return;
    }
    // This runs after UI focus, before gameplay Update. A modal consumes hotkeys
    // even when closing, so Escape cannot also unpause the underlying pause menu.
    let close = keyboard.just_pressed(KeyCode::Escape)
        || close_buttons
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    keyboard.clear();
    if close {
        if let Some(root) = state.root.take() {
            commands.entity(root).despawn();
        }
        state.finish(&mut session);
        return;
    }
    session.paused = true;
    for (interaction, choice) in &language_buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        *language = choice.0;
        state.save_failed = match crate::preferences::save_language(choice.0) {
            Ok(()) => false,
            Err(error) => {
                warn!("Could not save language preference: {error}");
                true
            }
        };
        break;
    }
}

fn update_tutorial_entry(
    tutorial: Option<Res<super::OnboardingState>>,
    mut buttons: Query<&mut Node, With<SettingsTutorial>>,
) {
    for mut node in &mut buttons {
        node.display = if tutorial.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn copy(kind: CopyKind, language: GameLanguage, failed: bool) -> &'static str {
    match kind {
        CopyKind::Title => language.pick("Settings", "设置"),
        CopyKind::Section => language.pick("INTERFACE LANGUAGE", "界面语言"),
        CopyKind::Intro => {
            language.pick("Language, card detail and audio.", "语言、选卡详情与声音。")
        }
        CopyKind::Selected => language.pick("Selected", "已选择"),
        CopyKind::SaveNotice => {
            if failed {
                language.pick(
                    "Your changes apply now, but could not be saved on this device.",
                    "更改已生效，但无法在此设备上保存偏好。",
                )
            } else {
                language.pick("Your choice is saved immediately. Your expedition stays paused while settings are open.", "选择后立即保存。设置页面打开期间，远征将保持暂停。")
            }
        }
        CopyKind::Back => language.pick("Back", "返回"),
        CopyKind::Tutorial => language.pick("How to play", "玩法指引"),
    }
}

fn update_settings_copy(
    language: Res<GameLanguage>,
    state: Res<SettingsState>,
    tutorial: Option<Res<super::OnboardingState>>,
    audio_save: Option<Res<super::AudioSaveStatus>>,
    #[cfg(not(target_arch = "wasm32"))] display_save: Option<
        Res<display_controls::DisplaySaveStatus>,
    >,
    mut labels: Query<(&mut Text, &SettingsCopy)>,
    mut selected: Query<(&SettingsSelected, &mut Visibility)>,
    mut launcher: Query<
        (&mut Visibility, &Interaction, &mut BackgroundColor),
        (
            With<SettingsOpen>,
            Without<SettingsSelected>,
            Without<SettingsLanguageButton>,
            Without<SettingsClose>,
        ),
    >,
    mut choices: Query<
        (
            &SettingsLanguageButton,
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        Without<SettingsOpen>,
    >,
    mut close: Query<
        (&Interaction, &mut BackgroundColor),
        (
            With<SettingsClose>,
            Without<SettingsOpen>,
            Without<SettingsLanguageButton>,
        ),
    >,
) {
    #[cfg(not(target_arch = "wasm32"))]
    let display_failed = display_save.as_ref().is_some_and(|save| save.failed);
    #[cfg(target_arch = "wasm32")]
    let display_failed = false;
    for (mut text, key) in &mut labels {
        text.set_if_neq(Text(
            copy(
                key.0,
                *language,
                state.save_failed
                    || tutorial.as_ref().is_some_and(|guide| guide.save_failed)
                    || audio_save.as_ref().is_some_and(|save| save.failed)
                    || display_failed,
            )
            .into(),
        ));
    }
    for (choice, mut visibility) in &mut selected {
        visibility.set_if_neq(if choice.0 == *language {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    for (mut visibility, interaction, mut background) in &mut launcher {
        visibility.set_if_neq(if state.open {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        });
        background
            .reborrow()
            .map_unchanged(|color| &mut color.0)
            .set_if_neq(if *interaction == Interaction::None {
                PANEL
            } else {
                Color::srgb(0.13, 0.18, 0.17)
            });
    }
    for (choice, interaction, mut background, mut border) in &mut choices {
        let selected = choice.0 == *language;
        border.set_if_neq(BorderColor::all(if selected {
            GOLD
        } else if *interaction != Interaction::None {
            IVORY
        } else {
            EDGE
        }));
        background
            .reborrow()
            .map_unchanged(|color| &mut color.0)
            .set_if_neq(if selected {
                Color::srgb(0.11, 0.16, 0.15)
            } else {
                Color::srgb(0.055, 0.085, 0.095)
            });
    }
    for (interaction, mut background) in &mut close {
        background
            .reborrow()
            .map_unchanged(|color| &mut color.0)
            .set_if_neq(if *interaction == Interaction::None {
                GOLD
            } else {
                Color::srgb(1.0, 0.85, 0.55)
            });
    }
}

#[derive(Clone, Copy)]
struct Layout {
    width: f32,
    height: f32,
    padding: f32,
    gap: f32,
    compact: bool,
}
fn layout(width: f32, height: f32) -> Layout {
    let compact = height < 540.0;
    Layout {
        width: (width - 40.0).min(720.0),
        height: (height - 24.0).min(640.0),
        padding: if compact { 12.0 } else { 24.0 },
        gap: if compact { 3.0 } else { 12.0 },
        compact,
    }
}

fn update_settings_layout(
    state: Res<SettingsState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    target: Option<Res<CaptureTarget>>,
    images: Option<Res<Assets<Image>>>,
    mut nodes: Query<(
        &mut Node,
        Has<SettingsPanel>,
        Has<SettingsHeader>,
        Has<SettingsLanguageRow>,
        Option<&SettingsCopy>,
    )>,
    tutorial: Option<Res<super::OnboardingState>>,
    audio_save: Option<Res<super::AudioSaveStatus>>,
    #[cfg(not(target_arch = "wasm32"))] display_save: Option<
        Res<display_controls::DisplaySaveStatus>,
    >,
    mut headings: Query<&mut TextFont, With<SettingsHeading>>,
) {
    if !state.open {
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    let display_failed = display_save.as_ref().is_some_and(|save| save.failed);
    #[cfg(target_arch = "wasm32")]
    let display_failed = false;
    let (width, height) = windows
        .single()
        .map(|window| (window.width(), window.height()))
        .unwrap_or_else(|_| {
            target
                .as_ref()
                .and_then(|target| images.as_ref()?.get(&target.0))
                .map(|image| (image.width() as f32, image.height() as f32))
                .unwrap_or((1280.0, 720.0))
        });
    let m = layout(width, height);
    for (mut node, panel, header, row, copy) in &mut nodes {
        if panel {
            node.reborrow()
                .map_unchanged(|node| &mut node.width)
                .set_if_neq(px(m.width));
            node.reborrow()
                .map_unchanged(|node| &mut node.height)
                .set_if_neq(Val::Auto);
            node.reborrow()
                .map_unchanged(|node| &mut node.max_height)
                .set_if_neq(px(m.height));
            node.reborrow()
                .map_unchanged(|node| &mut node.padding)
                .set_if_neq(UiRect::all(px(m.padding)));
            node.reborrow()
                .map_unchanged(|node| &mut node.row_gap)
                .set_if_neq(px(m.gap));
        }
        if header {
            node.reborrow()
                .map_unchanged(|node| &mut node.height)
                .set_if_neq(px(if m.compact { 28 } else { 60 }));
        }
        if let Some(copy) = copy
            && matches!(copy.0, CopyKind::Intro | CopyKind::SaveNotice)
        {
            let failed = state.save_failed
                || tutorial.as_ref().is_some_and(|guide| guide.save_failed)
                || audio_save.as_ref().is_some_and(|save| save.failed)
                || display_failed;
            let hide = m.compact && (matches!(copy.0, CopyKind::Intro) || !failed);
            node.reborrow()
                .map_unchanged(|node| &mut node.display)
                .set_if_neq(if hide { Display::None } else { Display::Flex });
        }
        if row {
            node.reborrow()
                .map_unchanged(|node| &mut node.height)
                .set_if_neq(px(if m.compact { 72 } else { 90 }));
        }
    }
    for heading in &mut headings {
        heading
            .map_unchanged(|font| &mut font.font_size)
            .set_if_neq(FontSize::Px(if m.compact { 24.0 } else { 34.0 }));
    }
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(SettingsState::default());
}

#[cfg(test)]
#[derive(Resource, Default)]
pub(super) struct TextRefreshProbe(pub usize);
#[cfg(test)]
pub(super) fn count_text_refreshes(
    changed: Query<Entity, Changed<Text>>,
    mut count: ResMut<TextRefreshProbe>,
) {
    count.0 = changed.iter().count();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Resource, Default)]
    struct LayoutRefreshProbe(usize);
    fn count_layout_refreshes(
        nodes: Query<Entity, Changed<Node>>,
        fonts: Query<Entity, Changed<TextFont>>,
        visibility: Query<Entity, Changed<Visibility>>,
        mut count: ResMut<LayoutRefreshProbe>,
    ) {
        count.0 = nodes.iter().count() + fonts.iter().count() + visibility.iter().count();
    }
    #[test]
    fn static_settings_do_not_dirty_text_layout_or_visibility() {
        let mut app = App::new();
        app.init_resource::<GameLanguage>()
            .insert_resource(SettingsState {
                open: true,
                ..default()
            })
            .init_resource::<TextRefreshProbe>()
            .init_resource::<LayoutRefreshProbe>()
            .add_systems(Update, (update_settings_copy, update_settings_layout))
            .add_systems(Last, (count_text_refreshes, count_layout_refreshes));
        app.world_mut().spawn((
            PrimaryWindow,
            Window {
                resolution: (1280, 720).into(),
                ..default()
            },
        ));
        let title = app
            .world_mut()
            .spawn((SettingsCopy(CopyKind::Title), Text::default()))
            .id();
        app.world_mut().spawn((SettingsPanel, Node::default()));
        app.world_mut()
            .spawn((SettingsHeading, TextFont::default()));
        let selected = app
            .world_mut()
            .spawn((SettingsSelected(GameLanguage::English), Visibility::Hidden))
            .id();
        app.update();
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 0);
        assert_eq!(app.world().resource::<LayoutRefreshProbe>().0, 0);
        app.world_mut()
            .insert_resource(GameLanguage::SimplifiedChinese);
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 1);
        assert_eq!(
            app.world().get::<Text>(title).unwrap().0,
            copy(CopyKind::Title, GameLanguage::SimplifiedChinese, false)
        );
        assert_eq!(
            *app.world().get::<Visibility>(selected).unwrap(),
            Visibility::Hidden
        );
        app.update();
        assert_eq!(app.world().resource::<TextRefreshProbe>().0, 0);
        assert_eq!(app.world().resource::<LayoutRefreshProbe>().0, 0);
    }

    #[test]
    fn settings_restores_battle_and_existing_modal_pause_ownership() {
        for already_paused in [false, true] {
            let mut session = GameSession {
                paused: already_paused,
                gold: 357,
                wood: 91,
                ..default()
            };
            let mut state = SettingsState::default();
            let seed = super::super::RunSeed(0x1234);
            let mut language = GameLanguage::English;
            state.begin(&mut session);
            assert!(state.open && session.paused);
            assert_eq!(language, GameLanguage::English);
            language = GameLanguage::SimplifiedChinese;
            state.finish(&mut session);
            assert!(!state.open);
            assert_eq!(session.paused, already_paused);
            assert_eq!(language, GameLanguage::SimplifiedChinese);
            assert_eq!((session.gold, session.wood, seed.0), (357, 91, 0x1234));
        }
    }

    #[test]
    fn escape_closes_settings_without_reaching_underlying_pause_hotkey() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameLanguage>()
            .insert_resource(GameSession {
                paused: true,
                ..default()
            })
            .insert_resource(SettingsState {
                open: true,
                prior_paused: true,
                ..default()
            })
            .add_systems(PreUpdate, handle_settings_input);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        assert!(!app.world().resource::<SettingsState>().open);
        assert!(app.world().resource::<GameSession>().paused);
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Escape)
        );
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Enter)
        );
    }

    #[test]
    fn settings_fits_desktop_and_compact_landscape() {
        for (width, height) in [
            (1760.0, 1350.0),
            (1280.0, 720.0),
            (844.0, 390.0),
            (667.0, 375.0),
        ] {
            let m = layout(width, height);
            assert!(m.width < width && m.height < height);
            let content = m.height - m.padding * 2.0;
            let required = if m.compact {
                28.0 + 18.0 + 72.0 + 36.0 + 76.0 + 28.0 + 48.0 + m.gap * 6.0
            } else {
                60.0 + 18.0 + 90.0 + 36.0 + 76.0 + 28.0 + 48.0 + m.gap * 6.0
            };
            assert!(
                required <= content,
                "settings content at {width}x{height}: {required}>{content}"
            );
        }
    }
    #[test]
    fn replay_entry_requires_plugin_and_restores_pause_before_requesting_guide() {
        for plugin_present in [false, true] {
            for prior_paused in [false, true] {
                let mut app = App::new();
                app.init_resource::<ButtonInput<KeyCode>>()
                    .init_resource::<GameLanguage>()
                    .insert_resource(GameSession {
                        paused: true,
                        ..default()
                    })
                    .insert_resource(SettingsState {
                        open: true,
                        prior_paused,
                        ..default()
                    })
                    .add_systems(PreUpdate, handle_settings_input)
                    .add_systems(Update, update_tutorial_entry);
                if plugin_present {
                    app.init_resource::<super::super::OnboardingState>();
                }
                let entry = app
                    .world_mut()
                    .spawn((SettingsTutorial, Interaction::Pressed, Node::default()))
                    .id();
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::Digit1);
                app.update();
                assert_eq!(
                    app.world().resource::<SettingsState>().open,
                    !plugin_present
                );
                assert_eq!(
                    app.world().resource::<GameSession>().paused,
                    if plugin_present { prior_paused } else { true }
                );
                assert_eq!(
                    app.world().get::<Node>(entry).unwrap().display,
                    if plugin_present {
                        Display::Flex
                    } else {
                        Display::None
                    }
                );
                assert!(
                    !app.world()
                        .resource::<ButtonInput<KeyCode>>()
                        .just_pressed(KeyCode::Digit1)
                );
                if plugin_present {
                    assert!(
                        app.world()
                            .resource::<super::super::OnboardingState>()
                            .replay_requested
                    );
                }
            }
        }
    }
    #[test]
    fn settings_replay_really_opens_over_chapters_and_results_and_stays_open() {
        for finished in [false, true] {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<GameLanguage>()
                .insert_resource(super::super::OnboardingState::default())
                .insert_resource(super::super::CampaignRun::default())
                .insert_resource(super::super::CampaignMenu(!finished))
                .insert_resource(super::super::AssetReadiness::default())
                .insert_resource(GameSession {
                    paused: true,
                    finished,
                    ..default()
                })
                .insert_resource(SettingsState {
                    open: true,
                    prior_paused: true,
                    ..default()
                })
                .add_systems(PreUpdate, handle_settings_input.in_set(SettingsInputSet));
            app.world_mut()
                .resource_mut::<super::super::AssetReadiness>()
                .ready = true;
            super::super::onboarding::install_input_test(&mut app);
            app.world_mut()
                .spawn((SettingsTutorial, Interaction::Pressed));
            app.update();
            assert!(!app.world().resource::<SettingsState>().open);
            assert_eq!(
                app.world().resource::<super::super::OnboardingState>().page,
                Some(0)
            );
            app.update();
            assert_eq!(
                app.world().resource::<super::super::OnboardingState>().page,
                Some(0)
            );
            assert!(app.world().resource::<GameSession>().paused);
            assert_eq!(
                app.world().resource::<super::super::CampaignMenu>().0,
                !finished
            );
            assert_eq!(app.world().resource::<GameSession>().finished, finished);
        }
    }
}
