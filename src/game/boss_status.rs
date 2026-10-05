//! Compact, unobstructed boss health and phase, separate from the round clock.
use crate::game::bosses::BossRuntime;
use crate::game::encounter_bosses::{ADVENTURE_BOSSES, AdventureBossState, EncounterBoss};
use crate::game::localization::{GameLanguage, boss_name};
use crate::game::mobile_layout::hud_metrics;
use crate::game::{BossEnemy, Enemy, GameSession};
use bevy::prelude::*;
#[derive(Component)]
struct BossStatus;
#[derive(Component)]
struct BossStatusPanel;
#[derive(Component)]
struct BossStatusTitle;
#[derive(Component)]
struct BossStatusBar;
pub(super) fn install(app: &mut App) {
    app.init_resource::<GameLanguage>()
        .init_resource::<BossRuntime>()
        .add_systems(Startup, spawn)
        .add_systems(Update, update);
}
fn spawn(mut commands: Commands) {
    commands
        .spawn((
            Name::new("Boss battle status"),
            BossStatus,
            Node {
                position_type: PositionType::Absolute,
                top: px(78),
                width: percent(100),
                justify_content: JustifyContent::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(6),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    BossStatusPanel,
                    Node {
                        width: px(260),
                        padding: UiRect::all(px(6)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(4),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.035, 0.045, 0.055, 0.94)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Name::new("Boss health title"),
                        BossStatusTitle,
                        Text::new(""),
                        TextFont {
                            font_size: FontSize::Px(14.0),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.81, 0.4)),
                    ));
                    parent
                        .spawn((
                            Node {
                                height: px(6),
                                width: percent(100),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.16, 0.12, 0.12)),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Name::new("Boss health fill"),
                                BossStatusBar,
                                Node {
                                    height: percent(100),
                                    width: percent(100),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.95, 0.25, 0.12)),
                            ));
                        });
                });
        });
}
fn title(language: GameLanguage, name: &str, hp: f32, max_hp: f32, enraged: bool) -> String {
    format!(
        "{} · {} · {:.0}%",
        name,
        language.pick(
            if enraged { "II" } else { "I" },
            if enraged { "二阶段" } else { "一阶段" }
        ),
        (hp / max_hp.max(1.0) * 100.0).clamp(0.0, 100.0)
    )
}
fn update(
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    windows: Query<&Window>,
    bosses: Query<(Entity, &BossEnemy, &Enemy, Option<&EncounterBoss>)>,
    adventures: Option<Res<AdventureBossState>>,
    runtime: Res<BossRuntime>,
    mut roots: Query<
        &mut Node,
        (
            With<BossStatus>,
            Without<BossStatusPanel>,
            Without<BossStatusBar>,
        ),
    >,
    mut panels: Query<
        &mut Node,
        (
            With<BossStatusPanel>,
            Without<BossStatus>,
            Without<BossStatusBar>,
        ),
    >,
    mut labels: Query<(&mut Text, &mut TextFont), With<BossStatusTitle>>,
    mut bars: Query<
        &mut Node,
        (
            With<BossStatusBar>,
            Without<BossStatus>,
            Without<BossStatusPanel>,
        ),
    >,
) {
    let boss = (!session.finished)
        .then(|| bosses.iter().find(|(_, _, enemy, _)| enemy.hp > 0.0))
        .flatten();
    let compact = windows
        .iter()
        .next()
        .is_some_and(|window| hud_metrics(window.width(), window.height()).compact);
    for mut node in &mut roots {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if boss.is_some() {
                Display::Flex
            } else {
                Display::None
            });
        node.reborrow()
            .map_unchanged(|value| &mut value.top)
            .set_if_neq(px(if compact { 120 } else { 78 }));
        node.reborrow()
            .map_unchanged(|value| &mut value.padding)
            .set_if_neq(UiRect::left(px(if compact { 72 } else { 0 })));
    }
    for mut node in &mut panels {
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(px(if compact { 190 } else { 260 }));
    }
    if let Some((entity, boss, enemy, adventure)) = boss {
        for (mut text, mut font) in &mut labels {
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(title(
                    *language,
                    adventure
                        .map_or_else(
                            || boss_name(*language, boss.index as usize),
                            |adventure| {
                                let def = &ADVENTURE_BOSSES[usize::from(adventure.index)];
                                language.pick(def.name_en, def.name_zh).to_string()
                            },
                        )
                        .as_str(),
                    enemy.hp,
                    enemy.max_hp,
                    runtime.is_enraged(entity),
                ));
            if adventure.is_some()
                && let Some(state) = adventures.as_ref()
            {
                text.push_str(&format!(
                    " · {:.0}{}",
                    state.remaining_secs.ceil(),
                    language.pick("s", "秒")
                ));
            }
            font.reborrow()
                .map_unchanged(|value| &mut value.font_size)
                .set_if_neq(FontSize::Px(if compact { 11.0 } else { 14.0 }));
        }
        for mut node in &mut bars {
            node.reborrow()
                .map_unchanged(|value| &mut value.width)
                .set_if_neq(percent(
                    (enemy.hp / enemy.max_hp.max(1.0) * 100.0).clamp(0.0, 100.0),
                ));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn localized_phase_and_real_health() {
        let en = title(
            GameLanguage::English,
            &boss_name(GameLanguage::English, 0),
            80.0,
            100.0,
            true,
        );
        assert!(en.contains("II · 80%"));
        assert!(!en.contains("阶段"));
        let zh = title(
            GameLanguage::SimplifiedChinese,
            &boss_name(GameLanguage::SimplifiedChinese, 0),
            100.0,
            100.0,
            false,
        );
        assert!(zh.contains("一阶段 · 100%"));
    }
    #[test]
    fn absent_boss_hides_the_panel() {
        let mut app = App::new();
        app.init_resource::<GameSession>();
        install(&mut app);
        app.update();
        let mut query = app.world_mut().query_filtered::<&Node, With<BossStatus>>();
        assert_eq!(query.single(app.world()).unwrap().display, Display::None);
    }
}
