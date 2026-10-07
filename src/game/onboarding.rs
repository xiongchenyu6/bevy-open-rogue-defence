//! Windowed, skippable first-expedition guide. Headless games opt out by omission.
use super::{AssetReadiness, Autoplay, CampaignMenu, CampaignRun, GameLanguage, GameSession};
use bevy::prelude::*;

pub struct OnboardingPlugin;
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OnboardingInputSet;
#[derive(Resource)]
pub struct OnboardingState {
    pub page: Option<usize>,
    pub replay_requested: bool,
    pub dismissed: bool,
    root: Option<Entity>,
    prior_paused: bool,
    replaying: bool,
    seen_run: u64,
    pub input_blocked: bool,
    pub save_failed: bool,
}
impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            page: None,
            replay_requested: false,
            dismissed: crate::preferences::load_tutorial_dismissed(),
            root: None,
            prior_paused: false,
            replaying: false,
            seen_run: 0,
            input_blocked: false,
            save_failed: false,
        }
    }
}
#[derive(Component, Clone, Copy)]
enum Action {
    Skip,
    Back,
    Next,
}
#[derive(Component)]
struct Guide;
impl Plugin for OnboardingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OnboardingState>()
            .add_systems(
                PreUpdate,
                input
                    .in_set(OnboardingInputSet)
                    .after(super::SettingsInputSet)
                    .after(super::asset_readiness::AssetReadinessInputSet)
                    .after(bevy::ui::UiSystems::Focus),
            )
            .add_systems(PostUpdate, draw);
    }
}
fn input(
    mut state: ResMut<OnboardingState>,
    mut session: ResMut<GameSession>,
    run: Res<CampaignRun>,
    menu: Res<CampaignMenu>,
    gate: Option<Res<AssetReadiness>>,
    autoplay: Option<Res<Autoplay>>,
    settings: Res<super::settings::SettingsState>,
    mut keys: ResMut<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    touches: Option<Res<Touches>>,
    buttons: Query<(&Interaction, &Action), Changed<Interaction>>,
    mut underlying: Query<&mut Interaction, Without<Action>>,
) {
    let held = mouse.as_ref().is_some_and(|m| m.pressed(MouseButton::Left))
        || touches.as_ref().is_some_and(|t| t.iter().next().is_some());
    if state.input_blocked && !held && state.page.is_none() {
        state.input_blocked = false;
    }
    let replay_ready =
        state.replay_requested && !settings.open && gate.as_ref().is_none_or(|g| g.ready);
    if (menu.0 || session.finished) && !state.replaying && !replay_ready {
        // Departure/result owns the new pause. Only discard our modal state.
        state.page = None;
        state.replay_requested = false;
    } else if state.page.is_none()
        && !settings.open
        && gate.as_ref().is_none_or(|g| g.ready)
        && (state.replay_requested
            || (!menu.0
                && !session.finished
                && autoplay.is_none()
                && !state.dismissed
                && run.run_id != state.seen_run))
    {
        state.seen_run = run.run_id;
        state.prior_paused = session.paused;
        state.page = Some(0);
        state.replaying = state.replay_requested && (menu.0 || session.finished);
        state.replay_requested = false;
        state.input_blocked = true;
    }
    if let Some(page) = state.page {
        session.paused = true;
        state.input_blocked = true;
        let action = if keys.just_pressed(KeyCode::Escape) {
            Some(Action::Skip)
        } else {
            buttons
                .iter()
                .find_map(|(i, a)| (*i == Interaction::Pressed).then_some(*a))
        };
        match action {
            Some(Action::Back) => state.page = Some(page.saturating_sub(1)),
            Some(Action::Next) if page < 3 => state.page = Some(page + 1),
            Some(Action::Skip | Action::Next) => {
                state.page = None;
                state.replaying = false;
                state.dismissed = true;
                state.save_failed = crate::preferences::save_tutorial_dismissed().is_err();
                session.paused = state.prior_paused;
            }
            None => {}
        }
    }
    if state.input_blocked {
        keys.clear();
        for mut interaction in &mut underlying {
            *interaction = Interaction::None;
        }
    }
}
fn copy(page: usize, language: GameLanguage) -> (&'static str, &'static str, &'static str) {
    match page {
        0 => (
            "dodge",
            language.pick("MOVE & DODGE", "移动与闪避"),
            language.pick(
                "Arrows / joystick move. Auto attacks.\nShift / dodge button avoids danger.",
                "方向键或摇杆移动，攻击自动施放。\nShift 或闪避按钮躲开危险。",
            ),
        ),
        1 => (
            "oath_banner",
            language.pick("THREE LIVES", "三次生命"),
            language.pick(
                "Three lives. Lose all three: defeat.\nGate countdown: unlimited rebuilds.",
                "英雄三次生命，耗尽后远征失败。\n城门倒下后按倒计时重建，次数不限。",
            ),
        ),
        2 => (
            "wood",
            language.pick("KEEP GROWING", "持续成长"),
            language.pick(
                "Wood: F bonds. Gold: G skills.\nKills: B market. Keep picking cards.",
                "木材选羁绊（F），金币学技能（G），战功逛黑市（B）。\n持续选择卡片，组合效果。",
            ),
        ),
        _ => (
            "tower",
            language.pick("YOUR LEGACY", "永久成长"),
            language.pick(
                "Win to unlock chapters. Expeditions earn shards.\nTraining gives permanent bonuses.",
                "通关解锁章节，远征获得碎片。\n在永久强化中花费碎片，提升每次远征。",
            ),
        ),
    }
}
fn draw(
    mut commands: Commands,
    mut state: ResMut<OnboardingState>,
    language: Res<GameLanguage>,
    server: Res<AssetServer>,
    mut drawn: Local<Option<(usize, GameLanguage)>>,
) {
    let desired = state.page.map(|p| (p, *language));
    if *drawn == desired && (desired.is_none() || state.root.is_some()) {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    *drawn = desired;
    let Some((page, language)) = desired else {
        return;
    };
    let (icon, title, body) = copy(page, language);
    let root = commands
        .spawn((
            Guide,
            Name::new("Tutorial overlay"),
            GlobalZIndex(900),
            bevy::ui::FocusPolicy::Block,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.03, 0.96)),
        ))
        .with_children(|root| {
            root.spawn(Node {
                width: percent(92),
                max_width: px(620),
                padding: UiRect::all(px(18)),
                row_gap: px(18),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|panel| {
                panel.spawn((
                    ImageNode::new(server.load(format!("ui/icons/{icon}.png"))),
                    Name::new("Tutorial illustration"),
                    Node {
                        width: px(64),
                        height: px(64),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
                panel.spawn((
                    Text::new(format!("{} / 4 · {}", page + 1, title)),
                    TextFont {
                        font_size: FontSize::Px(23.),
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.76, 0.45)),
                ));
                panel.spawn((
                    Text::new(body),
                    TextFont {
                        font_size: FontSize::Px(16.),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                ));
                panel
                    .spawn(Node {
                        width: percent(100),
                        column_gap: px(8),
                        ..default()
                    })
                    .with_children(|row| {
                        for (action, name, text) in [
                            (Action::Skip, "Tutorial skip", language.pick("Skip", "跳过")),
                            (Action::Back, "Tutorial back", language.pick("Back", "返回")),
                            (
                                Action::Next,
                                "Tutorial next",
                                if page == 3 {
                                    language.pick("Start", "开始")
                                } else {
                                    language.pick("Next", "下一步")
                                },
                            ),
                        ] {
                            if matches!(action, Action::Back) && page == 0 {
                                continue;
                            }
                            row.spawn((
                                Button,
                                action,
                                Name::new(name),
                                Node {
                                    flex_grow: 1.,
                                    min_height: px(48),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.16, 0.24, 0.27)),
                            ))
                            .with_child((
                                Text::new(text),
                                TextFont {
                                    font_size: FontSize::Px(16.),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                            ));
                        }
                    });
            });
        })
        .id();
    state.root = Some(root);
}
pub(crate) fn reset_run(world: &mut World) {
    if let Some(mut state) = world.get_resource_mut::<OnboardingState>() {
        state.page = None;
        state.root = None;
        state.replay_requested = false;
        state.replaying = false;
        state.input_blocked = false;
    }
}

#[cfg(test)]
pub(crate) fn install_input_test(app: &mut App) {
    app.add_systems(PreUpdate, input.after(super::SettingsInputSet));
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(OnboardingState {
            dismissed: false,
            ..default()
        })
        .insert_resource(GameSession::default())
        .insert_resource(CampaignRun {
            stage: 0,
            run_id: 1,
        })
        .insert_resource(CampaignMenu(false))
        .insert_resource(super::super::settings::SettingsState::default())
        .insert_resource(AssetReadiness::default())
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .add_systems(PreUpdate, input);
        app
    }
    #[test]
    fn first_expedition_waits_for_assets_and_blocks_underlying_choices_and_keys() {
        let mut app = app();
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, None);
        app.world_mut().resource_mut::<AssetReadiness>().ready = true;
        let choice = app.world_mut().spawn(Interaction::Pressed).id();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit1);
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, Some(0));
        assert!(app.world().resource::<GameSession>().paused);
        assert_eq!(
            *app.world().get::<Interaction>(choice).unwrap(),
            Interaction::None
        );
        assert!(
            !app.world()
                .resource::<ButtonInput<KeyCode>>()
                .just_pressed(KeyCode::Digit1)
        );
        let next = app
            .world_mut()
            .spawn((Action::Next, Interaction::Pressed))
            .id();
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, Some(1));
        assert!(!app.world().resource::<OnboardingState>().dismissed);
        app.world_mut().despawn(next);
    }
    #[test]
    fn dismissed_runs_stay_quiet_replay_preserves_pause_and_menu_cleans_up() {
        let mut app = app();
        app.world_mut().resource_mut::<AssetReadiness>().ready = true;
        app.world_mut().resource_mut::<OnboardingState>().dismissed = true;
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, None);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<OnboardingState>()
            .replay_requested = true;
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, Some(0));
        app.world_mut().resource_mut::<CampaignMenu>().0 = true;
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, None);
        assert!(app.world().resource::<GameSession>().paused);
        assert!(!app.world().resource::<OnboardingState>().replay_requested);
    }
    #[test]
    fn autoplay_never_triggers_and_reset_drops_transient_state() {
        let mut app = app();
        app.world_mut().resource_mut::<AssetReadiness>().ready = true;
        app.insert_resource(Autoplay);
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, None);
        app.world_mut().remove_resource::<Autoplay>();
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, Some(0));
        reset_run(app.world_mut());
        assert_eq!(app.world().resource::<OnboardingState>().page, None);
        assert!(!app.world().resource::<OnboardingState>().input_blocked);
    }
    #[test]
    fn departure_keeps_the_campaign_pause_after_a_running_replay() {
        let mut app = app();
        app.world_mut().resource_mut::<AssetReadiness>().ready = true;
        app.world_mut().resource_mut::<OnboardingState>().dismissed = true;
        app.world_mut()
            .resource_mut::<OnboardingState>()
            .replay_requested = true;
        app.update();
        assert!(!app.world().resource::<OnboardingState>().prior_paused);
        app.world_mut().resource_mut::<CampaignMenu>().0 = true;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().resource::<OnboardingState>().page, None);
        assert!(app.world().resource::<GameSession>().paused);
    }
}
