//! One compact native display button and F11, both backed by the real Window.
#[path = "display_preferences.rs"]
mod preferences;
use super::{EDGE, GOLD, IVORY, PANEL, SettingsInputSet, SettingsState, label};
use crate::game::GameLanguage;
use bevy::{
    prelude::*,
    window::{MonitorSelection, PrimaryWindow, WindowMode, WindowPosition, WindowResolution},
};
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisplayPreferences {
    pub fullscreen: bool,
}
impl Default for DisplayPreferences {
    fn default() -> Self {
        Self {
            fullscreen: preferences::load().unwrap_or(false),
        }
    }
}
#[derive(Resource, Default, Debug)]
pub struct DisplaySaveStatus {
    pub failed: bool,
}
pub struct DisplayControlsPlugin;
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
struct WindowedState(Option<(WindowResolution, WindowPosition)>);
#[derive(Component)]
struct Toggle;
#[derive(Component)]
struct DisplayCopy;
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
struct Saved {
    value: bool,
    path: Option<std::path::PathBuf>,
}
#[cfg(not(target_arch = "wasm32"))]
impl FromWorld for Saved {
    fn from_world(world: &mut World) -> Self {
        Self {
            value: world.resource::<DisplayPreferences>().fullscreen,
            path: preferences::path(),
        }
    }
}
impl Plugin for DisplayControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DisplayPreferences>()
            .init_resource::<DisplaySaveStatus>();
        #[cfg(not(target_arch = "wasm32"))]
        app.init_resource::<WindowedState>()
            .init_resource::<Saved>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(
                PreUpdate,
                (
                    shortcut
                        .after(bevy::input::InputSystems)
                        .before(SettingsInputSet)
                        .before(super::SettingsDepartureInputSet),
                    button.after(SettingsInputSet),
                ),
            )
            .add_systems(Update, (apply_mode, persist, update_copy).chain());
    }
}
pub(super) fn spawn(parent: &mut ChildSpawnerCommands) {
    #[cfg(not(target_arch = "wasm32"))]
    parent
        .spawn((
            Name::new("Settings display toggle"),
            Toggle,
            Button,
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                min_width: px(0),
                height: px(48),
                border: UiRect::all(px(1)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(EDGE),
        ))
        .with_children(|button| {
            button.spawn((
                DisplayCopy,
                label("", 13.0, IVORY),
                TextLayout::justify(Justify::Center),
            ));
        });
    #[cfg(target_arch = "wasm32")]
    let _ = parent;
}
#[cfg(not(target_arch = "wasm32"))]
fn shortcut(mut keyboard: ResMut<ButtonInput<KeyCode>>, mut prefs: ResMut<DisplayPreferences>) {
    if keyboard.just_pressed(KeyCode::F11) {
        prefs.fullscreen = !prefs.fullscreen;
        keyboard.clear_just_pressed(KeyCode::F11);
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn button(
    state: Res<SettingsState>,
    mut prefs: ResMut<DisplayPreferences>,
    mut buttons: Query<
        (&Interaction, &mut BackgroundColor, &mut BorderColor),
        (With<Toggle>, Changed<Interaction>),
    >,
) {
    for (interaction, mut background, mut border) in &mut buttons {
        background.0 = if *interaction == Interaction::None {
            PANEL
        } else {
            Color::srgb(0.15, 0.12, 0.07)
        };
        border.set_all(if *interaction == Interaction::None {
            EDGE
        } else {
            GOLD
        });
        if state.open && *interaction == Interaction::Pressed {
            prefs.fullscreen = !prefs.fullscreen;
            break;
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn apply_mode(
    prefs: Res<DisplayPreferences>,
    mut restore: ResMut<WindowedState>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let desired = if prefs.fullscreen {
        WindowMode::BorderlessFullscreen(MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    for mut window in &mut windows {
        if window.mode != desired {
            if prefs.fullscreen && window.mode == WindowMode::Windowed {
                restore.0 = Some((window.resolution.clone(), window.position));
            }
            if !prefs.fullscreen
                && let Some((resolution, position)) = restore.0.take()
            {
                window.resolution = resolution;
                window.position = position;
            }
            window.mode = desired;
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn persist(
    prefs: Res<DisplayPreferences>,
    mut saved: ResMut<Saved>,
    mut status: ResMut<DisplaySaveStatus>,
) {
    if prefs.fullscreen == saved.value {
        return;
    }
    status.failed = saved
        .path
        .as_deref()
        .ok_or_else(|| "No user configuration directory".to_string())
        .and_then(|path| preferences::save_at(path, prefs.fullscreen))
        .is_err();
    saved.value = prefs.fullscreen;
}
#[cfg(not(target_arch = "wasm32"))]
fn update_copy(
    prefs: Res<DisplayPreferences>,
    language: Res<GameLanguage>,
    mut copy: Query<&mut Text, With<DisplayCopy>>,
) {
    for mut text in &mut copy {
        **text = if prefs.fullscreen {
            language.pick("Fullscreen", "全屏")
        } else {
            language.pick("Windowed", "窗口")
        }
        .to_string();
    }
}
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    fn app(path: std::path::PathBuf) -> (App, Entity) {
        let mut app = App::new();
        app.insert_resource(DisplayPreferences { fullscreen: false })
            .init_resource::<SettingsState>()
            .init_resource::<GameLanguage>()
            .add_plugins(DisplayControlsPlugin);
        app.world_mut().resource_mut::<Saved>().path = Some(path);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        (app, window)
    }
    #[test]
    fn real_button_changes_window_mode_preserves_size_and_saves() {
        let dir = std::env::temp_dir().join(format!("rogue-display-button-{}", std::process::id()));
        let path = dir.join("display");
        let (mut app, window) = app(path.clone());
        let before = app
            .world()
            .get::<Window>(window)
            .unwrap()
            .resolution
            .clone();
        app.world_mut().resource_mut::<SettingsState>().open = true;
        app.world_mut().spawn((
            Toggle,
            Interaction::Pressed,
            BackgroundColor(PANEL),
            BorderColor::all(EDGE),
        ));
        app.update();
        assert_eq!(
            app.world().get::<Window>(window).unwrap().mode,
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        );
        assert_eq!(
            app.world().get::<Window>(window).unwrap().resolution,
            before
        );
        assert_eq!(std::fs::read_to_string(path).unwrap().trim(), "fullscreen");
        assert!(!app.world().resource::<DisplaySaveStatus>().failed);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(1920.0, 1080.0);
        app.world_mut()
            .resource_mut::<DisplayPreferences>()
            .fullscreen = false;
        app.update();
        assert_eq!(
            app.world().get::<Window>(window).unwrap().mode,
            WindowMode::Windowed
        );
        assert_eq!(
            app.world().get::<Window>(window).unwrap().resolution,
            before,
            "fullscreen backend size must not overwrite saved window size"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn f11_runs_before_settings_clears_keyboard_and_write_failure_is_visible() {
        let dir = std::env::temp_dir().join(format!("rogue-display-f11-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (mut app, window) = app(dir.clone());
        app.add_systems(
            PreUpdate,
            (|mut keys: ResMut<ButtonInput<KeyCode>>| keys.clear()).in_set(SettingsInputSet),
        );
        app.add_plugins(bevy::input::InputPlugin);
        let event = |state| bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::F11,
            logical_key: bevy::input::keyboard::Key::F11,
            state,
            text: None,
            repeat: false,
            window,
        };
        app.world_mut()
            .write_message(event(bevy::input::ButtonState::Pressed));
        app.update();
        assert!(app.world().resource::<DisplayPreferences>().fullscreen);
        assert_eq!(
            app.world().get::<Window>(window).unwrap().mode,
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        );
        assert!(app.world().resource::<DisplaySaveStatus>().failed);
        app.update();
        assert!(
            app.world().resource::<DisplayPreferences>().fullscreen,
            "held F11 cannot toggle repeatedly"
        );
        app.world_mut()
            .write_message(event(bevy::input::ButtonState::Released));
        app.update();
        app.world_mut()
            .write_message(event(bevy::input::ButtonState::Pressed));
        app.update();
        assert!(!app.world().resource::<DisplayPreferences>().fullscreen);
        assert_eq!(
            app.world().get::<Window>(window).unwrap().mode,
            WindowMode::Windowed
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
