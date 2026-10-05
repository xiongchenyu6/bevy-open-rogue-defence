//! Explicit, cancellable departure from a running expedition.
use super::{EDGE, GOLD, IVORY, PANEL, SettingsInputSet, SettingsState, label};
use crate::game::{GameLanguage, GameSession};
use bevy::prelude::*;
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SettingsDepartureInputSet;
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Chapters,
    Quit,
    Cancel,
    Confirm,
}
#[derive(Resource, Default)]
struct Departure {
    choice: Option<Action>,
    confirmed: Option<Action>,
}
#[derive(Component)]
struct ActionCopy(Action);
pub(super) fn install(app: &mut App) {
    app.init_resource::<Departure>()
        .add_systems(
            PreUpdate,
            input
                .in_set(SettingsDepartureInputSet)
                .before(SettingsInputSet)
                .after(bevy::ui::UiSystems::Focus),
        )
        .add_systems(Update, (apply, update_copy).chain());
}
fn text(action: Action, language: GameLanguage) -> &'static str {
    match action {
        Action::Chapters => language.pick("Chapters", "章节"),
        Action::Quit => language.pick("Quit", "退出"),
        Action::Cancel => language.pick("Keep playing", "继续远征"),
        Action::Confirm => language.pick("Leave expedition", "离开远征"),
    }
}
fn button(parent: &mut ChildSpawnerCommands, action: Action, name: &str) {
    parent
        .spawn((
            Button,
            action,
            Name::new(name.to_owned()),
            Node {
                flex_basis: px(0),
                flex_grow: 1.0,
                min_width: px(0),
                height: px(48),
                min_height: px(48),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(PANEL),
            BorderColor::all(GOLD),
        ))
        .with_children(|b| {
            b.spawn((ActionCopy(action), label("", 13.0, IVORY)));
        });
}
pub(super) fn spawn(parent: &mut ChildSpawnerCommands) {
    button(parent, Action::Chapters, "Settings chapters");
    #[cfg(not(target_arch = "wasm32"))]
    button(parent, Action::Quit, "Settings quit");
}
fn update_copy(
    language: Res<GameLanguage>,
    departure: Res<Departure>,
    mut labels: Query<(&mut Text, &ActionCopy)>,
) {
    for (mut label, key) in &mut labels {
        **label = if key.0 == Action::Confirm {
            if departure.choice == Some(Action::Quit) {
                language.pick("Quit game", "退出游戏")
            } else {
                language.pick("Return to chapters", "返回章节")
            }
        } else {
            text(key.0, *language)
        }
        .into();
    }
}
fn input(
    mut commands: Commands,
    mut state: ResMut<SettingsState>,
    mut departure: ResMut<Departure>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    buttons: Query<(&Action, &Interaction), Changed<Interaction>>,
) {
    if !state.open {
        return;
    }
    if state.confirmation.is_some() && keys.just_pressed(KeyCode::Escape) {
        keys.clear();
        cancel(&mut commands, &mut state, &mut departure);
        return;
    }
    for (action, interaction) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            Action::Cancel => {
                cancel(&mut commands, &mut state, &mut departure);
            }
            Action::Confirm if state.confirmation.is_some() => {
                departure.confirmed = departure.choice.take();
            }
            Action::Chapters | Action::Quit if state.confirmation.is_none() => {
                departure.choice = Some(*action);
                state.confirmation =
                    Some(prompt(&mut commands, *language, session.finished, *action));
            }
            _ => {}
        }
        break;
    }
}
fn cancel(commands: &mut Commands, state: &mut SettingsState, departure: &mut Departure) {
    if let Some(root) = state.confirmation.take() {
        commands.entity(root).despawn();
    }
    departure.choice = None;
}
fn prompt(
    commands: &mut Commands,
    language: GameLanguage,
    finished: bool,
    action: Action,
) -> Entity {
    commands.spawn((Name::new("Settings departure confirmation"),GlobalZIndex(530),
        bevy::ui::FocusPolicy::Block, BackgroundColor(Color::srgba(0.0,0.0,0.0,0.92)),
        Node{position_type:PositionType::Absolute,width:percent(100),height:percent(100),
            align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()}))
    .with_children(|root| {
        root.spawn((Node{width:percent(90),max_width:px(520),padding:UiRect::all(px(20)),
            flex_direction:FlexDirection::Column,row_gap:px(16),border:UiRect::all(px(1)),..default()},
            BackgroundColor(PANEL),BorderColor::all(EDGE)))
        .with_children(|panel| {
            panel.spawn(label(if action==Action::Quit {language.pick("Quit the game?","退出游戏？")}
                else {language.pick("Return to chapters?","返回章节？")},24.0,IVORY));
            panel.spawn(label(if finished {language.pick("Your saved campaign progress is kept.","已保存的战役进度会保留。")}
                else {language.pick("This expedition cannot be resumed. Its build and unspent resources will be lost. Saved campaign progress is kept.","本次远征无法继续，构筑与未使用资源将丢失。已保存的战役进度会保留。")},14.0,IVORY));
            panel.spawn(Node{column_gap:px(12),..default()}).with_children(|row| {
                button(row,Action::Cancel,"Settings departure cancel");
                button(row,Action::Confirm,"Settings departure confirm");
            });
        });
    }).id()
}
fn apply(world: &mut World) {
    let Some(choice) = world.resource_mut::<Departure>().confirmed.take() else {
        return;
    };
    if choice == Action::Chapters {
        crate::game::campaign_ui::open_chapters(world);
        world.insert_resource(Departure::default());
    } else if choice == Action::Quit {
        world.write_message(AppExit::Success);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn departure_requires_confirmation_and_escape_preserves_paused_run() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameLanguage>()
            .insert_resource(GameSession {
                paused: true,
                gold: 321,
                ..default()
            })
            .insert_resource(SettingsState {
                open: true,
                ..default()
            });
        install(&mut app);
        let button = app
            .world_mut()
            .spawn((Action::Quit, Interaction::Pressed))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<Departure>().choice,
            Some(Action::Quit)
        );
        assert!(app.world().resource::<Departure>().confirmed.is_none());
        assert!(
            app.world()
                .resource::<SettingsState>()
                .confirmation
                .is_some()
        );
        app.world_mut().entity_mut(button).insert(Interaction::None);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(
            app.world()
                .resource::<SettingsState>()
                .confirmation
                .is_none()
        );
        assert!(app.world().resource::<Departure>().choice.is_none());
        assert!(app.world().resource::<SettingsState>().open);
        assert!(app.world().resource::<GameSession>().paused);
        assert_eq!(app.world().resource::<GameSession>().gold, 321);
    }
    #[test]
    fn confirmed_quit_emits_the_real_app_exit_message() {
        let mut app = App::new();
        app.add_message::<AppExit>()
            .init_resource::<Departure>()
            .add_systems(Update, apply);
        app.world_mut().resource_mut::<Departure>().confirmed = Some(Action::Quit);
        app.update();
        assert_eq!(app.world().resource::<Messages<AppExit>>().len(), 1);
        app.update();
        assert_eq!(app.world().resource::<Messages<AppExit>>().len(), 1);
    }
}
