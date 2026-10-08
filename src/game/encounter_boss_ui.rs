//! Optional camp battles always offer a safe exit; all previews come from the battle catalog.
use super::encounter_bosses::{
    ADVENTURE_BATTLE_SECONDS, ADVENTURE_BOSSES, ADVENTURE_OFFER_TIMES, ADVENTURE_SITES,
    AdventureBossChoice, AdventureBossState, AdventureCampRequest,
};
use super::{GameLanguage, GameSession, HeroVitals};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
    ui::UiSystems,
};

pub(crate) struct EncounterBossUiPlugin;
impl Plugin for EncounterBossUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EncounterBossPanelState>()
            .init_resource::<AdventureBossState>()
            .init_resource::<GameSession>()
            .init_resource::<GameLanguage>()
            .init_resource::<HeroVitals>()
            .init_resource::<Touches>()
            .init_resource::<RelicToastState>()
            .add_message::<MouseWheel>()
            .add_message::<AdventureBossChoice>()
            .add_message::<AdventureCampRequest>()
            .add_systems(Startup, camp_entries)
            .add_systems(PostUpdate, position_entries)
            .add_systems(PreUpdate, (input, enter_camp).after(UiSystems::Focus))
            .add_systems(Update, (view, scroll, relic_feedback));
    }
}
#[derive(Resource, Default)]
pub(crate) struct EncounterBossPanelState {
    root: Option<Entity>,
    index: Option<u8>,
    language: Option<GameLanguage>,
    size: Vec2,
}
#[derive(Component)]
struct Choice(bool);
#[derive(Component)]
struct PreviewScroll;
fn input(
    state: Res<AdventureBossState>,
    session: Res<GameSession>,
    buttons: Query<(&Choice, &Interaction), Changed<Interaction>>,
    mut choices: MessageWriter<AdventureBossChoice>,
) {
    if state.pending.is_none() || session.finished {
        return;
    }
    if let Some((button, _)) = buttons
        .iter()
        .find(|(_, interaction)| **interaction == Interaction::Pressed)
    {
        choices.write(if button.0 {
            AdventureBossChoice::Fight
        } else {
            AdventureBossChoice::TakeReward
        });
    }
}
fn risk(language: GameLanguage, lives: u32) -> String {
    if language.is_chinese() {
        format!(
            "剩余生命 {lives} · 战败会消耗正常生命；最后一命耗尽，远征失败。超时 {ADVENTURE_BATTLE_SECONDS:.0} 秒撤退，无战斗奖励。"
        )
    } else {
        format!(
            "{lives} lives left. Enemy damage can cost lives; losing your last life ends the run. Retreat after {ADVENTURE_BATTLE_SECONDS:.0}s gives no battle reward."
        )
    }
}
fn view(
    mut commands: Commands,
    mut panel: ResMut<EncounterBossPanelState>,
    state: Res<AdventureBossState>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    vitals: Res<HeroVitals>,
    assets: Option<Res<AssetServer>>,
    windows: Query<&Window>,
) {
    let index = state.pending.filter(|_| !session.finished);
    let size = windows
        .iter()
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |window| {
            Vec2::new(window.width(), window.height())
        });
    if index == panel.index && panel.language == Some(*language) && size == panel.size {
        return;
    }
    if let Some(root) = panel.root.take() {
        commands.entity(root).try_despawn();
    }
    panel.index = index;
    panel.language = Some(*language);
    panel.size = size;
    let (Some(index), Some(assets)) = (index, assets) else {
        return;
    };
    let Some(def) = ADVENTURE_BOSSES.get(usize::from(index)) else {
        return;
    };
    let compact = size.y < 540.0;
    let narrow = size.x < 620.0;
    let heading = language.pick(def.name_en, def.name_zh);
    let root = commands
        .spawn((
            Name::new("Encounter boss choice"),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(px(if compact { 12 } else { 28 })),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            GlobalZIndex(285),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px((size.x - 24.0).min(860.0)),
                        max_height: px(size.y - 24.0),
                        padding: UiRect::all(px(if compact { 12 } else { 22 })),
                        border: UiRect::all(px(1)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(if compact { 8 } else { 14 }),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.025, 0.045, 0.05)),
                    BorderColor::all(Color::srgb(0.8, 0.62, 0.3)),
                ))
                .with_children(|body| {
                    body.spawn(Node {
                        column_gap: px(14),
                        align_items: AlignItems::Center,
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|header| {
                        header.spawn((
                            ImageNode::new(assets.load(format!("ui/{}", def.icon_path))),
                            Node {
                                width: px(if compact { 48 } else { 76 }),
                                height: px(if compact { 48 } else { 76 }),
                                flex_shrink: 0.0,
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        header
                            .spawn(Node {
                                flex_direction: FlexDirection::Column,
                                row_gap: px(4),
                                ..default()
                            })
                            .with_children(|copy| {
                                copy.spawn((
                                    Text::new(
                                        language
                                            .pick("OPTIONAL CAMP CHALLENGE", "营地奇遇 · 自选挑战"),
                                    ),
                                    TextFont {
                                        font_size: FontSize::Px(11.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.8, 0.63, 0.35)),
                                ));
                                copy.spawn((
                                    Name::new("Encounter boss title"),
                                    Text::new(heading),
                                    TextFont {
                                        font_size: FontSize::Px(if compact { 20.0 } else { 28.0 }),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.98, 0.94, 0.8)),
                                ));
                                copy.spawn((
                                    Text::new(format!(
                                        "{} {:.0} · {} {:.0}",
                                        language.pick("Health", "生命"),
                                        def.hp,
                                        language.pick("Attack", "攻击"),
                                        def.damage
                                    )),
                                    TextFont {
                                        font_size: FontSize::Px(12.0),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.7, 0.78, 0.78)),
                                ));
                            });
                    });
                    body.spawn((
                        Name::new("Encounter boss risk"),
                        Text::new(risk(*language, vitals.lives_remaining())),
                        TextFont {
                            font_size: FontSize::Px(if compact { 11.0 } else { 13.0 }),
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 0.65, 0.4)),
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    body.spawn((
                        PreviewScroll,
                        ScrollPosition::default(),
                        Node {
                            min_height: px(0),
                            max_height: px(if compact { 130 } else { 270 }),
                            overflow: Overflow::scroll_y(),
                            flex_direction: if narrow {
                                FlexDirection::Column
                            } else {
                                FlexDirection::Row
                            },
                            column_gap: px(10),
                            row_gap: px(8),
                            ..default()
                        },
                    ))
                    .with_children(|skills| {
                        for (i, ability) in def.abilities.iter().enumerate() {
                            skills
                                .spawn((
                                    Name::new(format!("Encounter boss skill {i}")),
                                    Node {
                                        width: if narrow { percent(100) } else { percent(32.2) },
                                        flex_shrink: 0.0,
                                        padding: UiRect::all(px(8)),
                                        border: UiRect::all(px(1)),
                                        flex_direction: FlexDirection::Column,
                                        row_gap: px(5),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.055, 0.075, 0.08)),
                                    BorderColor::all(Color::srgb(0.2, 0.29, 0.29)),
                                ))
                                .with_children(|card| {
                                    card.spawn((
                                        Text::new(language.pick(ability.name_en, ability.name_zh)),
                                        TextFont {
                                            font_size: FontSize::Px(if compact {
                                                12.0
                                            } else {
                                                15.0
                                            }),
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.95, 0.8, 0.5)),
                                    ));
                                    card.spawn((
                                        Name::new(format!("Encounter boss skill description {i}")),
                                        Text::new(
                                            language.pick(
                                                ability.description_en,
                                                ability.description_zh,
                                            ),
                                        ),
                                        TextFont {
                                            font_size: FontSize::Px(if compact {
                                                10.5
                                            } else {
                                                12.0
                                            }),
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.76, 0.82, 0.81)),
                                    ));
                                });
                        }
                    });
                    body.spawn((
                        Name::new("Encounter boss victory reward"),
                        Text::new(format!(
                            "{} · {} {} · {} {} · {} {} · {}\n{} · {}",
                            language.pick("Victory", "击败奖励"),
                            def.reward_gold,
                            language.pick("gold", "金币"),
                            def.reward_wood,
                            language.pick("wood", "木材"),
                            def.reward_xp,
                            language.pick("XP", "经验"),
                            language.pick("1 transmutation fruit", "1 个转职果实"),
                            super::basic_relics::relic_name(index, *language),
                            super::basic_relics::relic_description(index, *language)
                        )),
                        TextFont {
                            font_size: FontSize::Px(if compact { 11.0 } else { 13.0 }),
                            ..default()
                        },
                        TextColor(Color::srgb(0.85, 0.74, 0.48)),
                        Node {
                            flex_shrink: 0.0,
                            ..default()
                        },
                    ));
                    body.spawn(Node {
                        flex_direction: if narrow {
                            FlexDirection::Column
                        } else {
                            FlexDirection::Row
                        },
                        column_gap: px(10),
                        row_gap: px(7),
                        flex_shrink: 0.0,
                        ..default()
                    })
                    .with_children(|footer| {
                        for (fight, label, color) in [
                            (
                                true,
                                language
                                    .pick("Fight for the full reward", "挑战 · 赢取完整奖励")
                                    .to_string(),
                                Color::srgb(0.7, 0.39, 0.13),
                            ),
                            (
                                false,
                                format!(
                                    "{} · {} {} / {} {}",
                                    language.pick("Take & leave", "领取并离开"),
                                    def.small_gold,
                                    language.pick("gold", "金币"),
                                    def.small_wood,
                                    language.pick("wood", "木材")
                                ),
                                Color::srgb(0.09, 0.27, 0.24),
                            ),
                        ] {
                            footer
                                .spawn((
                                    Button,
                                    Choice(fight),
                                    Name::new(if fight {
                                        "Encounter boss fight"
                                    } else {
                                        "Encounter boss safe reward"
                                    }),
                                    Node {
                                        width: if narrow { percent(100) } else { percent(49) },
                                        min_height: px(42),
                                        padding: UiRect::all(px(8)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    BackgroundColor(color),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new(label),
                                        TextFont {
                                            font_size: FontSize::Px(if compact {
                                                12.0
                                            } else {
                                                15.0
                                            }),
                                            ..default()
                                        },
                                        TextColor(Color::srgb(0.98, 0.94, 0.82)),
                                        Pickable::IGNORE,
                                    ));
                                });
                        }
                    });
                });
        })
        .id();
    panel.root = Some(root);
}
fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    mut bodies: Query<&mut ScrollPosition, With<PreviewScroll>>,
) {
    let delta = wheel
        .read()
        .map(|wheel| {
            -wheel.y
                * if wheel.unit == MouseScrollUnit::Line {
                    24.0
                } else {
                    1.0
                }
        })
        .sum::<f32>()
        + touches.iter().map(|touch| -touch.delta().y).sum::<f32>();
    for mut body in &mut bodies {
        body.y = (body.y + delta).max(0.0);
    }
}
pub(crate) fn reset_run(world: &mut World) {
    if let Some(root) = world
        .get_resource_mut::<EncounterBossPanelState>()
        .and_then(|mut panel| panel.root.take())
        && let Ok(entity) = world.get_entity_mut(root)
    {
        entity.despawn();
    }
    world.insert_resource(EncounterBossPanelState::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn risk_is_single_language_and_explains_last_life_and_real_timeout() {
        let en = risk(GameLanguage::English, 1);
        assert!(en.contains("last life"));
        assert!(en.contains("90s"));
        assert!(!en.contains("生命"));
        let zh = risk(GameLanguage::SimplifiedChinese, 1);
        assert!(zh.contains("最后一命"));
        assert!(zh.contains("90 秒"));
    }
    #[test]
    fn reset_removes_modal_without_changing_battle_pause() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        world.insert_resource(EncounterBossPanelState {
            root: Some(root),
            ..default()
        });
        world.insert_resource(GameSession {
            paused: true,
            ..default()
        });
        reset_run(&mut world);
        assert!(world.resource::<EncounterBossPanelState>().root.is_none());
        assert!(world.resource::<GameSession>().paused);
        assert!(world.get_entity(root).is_err());
    }
}

#[derive(Component)]
struct CampEntry(u8);
#[derive(Component)]
struct CampEntryCaption(u8);
fn camp_entries(mut commands: Commands, assets: Option<Res<AssetServer>>) {
    let Some(assets) = assets else {
        return;
    };
    for (index, def) in ADVENTURE_BOSSES.iter().enumerate() {
        commands
            .spawn((
                Name::new(format!("Adventure camp entry {index}")),
                CampEntry(index as u8),
                Button,
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    width: px(128),
                    min_height: px(40),
                    padding: UiRect::all(px(5)),
                    column_gap: px(5),
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.025, 0.04, 0.04, 0.9)),
                BorderColor::all(Color::srgb(0.7, 0.48, 0.2)),
                GlobalZIndex(80),
            ))
            .with_children(|parent| {
                parent.spawn((
                    ImageNode::new(assets.load(format!("ui/{}", def.icon_path))),
                    Node {
                        width: px(28),
                        height: px(28),
                        ..default()
                    },
                ));
                parent.spawn((
                    CampEntryCaption(index as u8),
                    Text::new(""),
                    Node::default(),
                    BackgroundColor(Color::NONE),
                    TextFont {
                        font_size: FontSize::Px(11.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.94, 0.86, 0.67)),
                ));
            });
    }
}
fn enter_camp(
    state: Res<AdventureBossState>,
    session: Res<GameSession>,
    buttons: Query<(&CampEntry, &Interaction), Changed<Interaction>>,
    mut requests: MessageWriter<AdventureCampRequest>,
) {
    if session.paused || session.finished || state.pending.is_some() || state.active.is_some() {
        return;
    }
    if let Some((entry, _)) = buttons.iter().find(|(entry, i)| {
        **i == Interaction::Pressed
            && state.unlocked & (1 << entry.0) != 0
            && state.resolved & (1 << entry.0) == 0
    }) {
        requests.write(AdventureCampRequest(entry.0));
    }
}
fn position_entries(
    state: Res<AdventureBossState>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    mut entries: Query<(&CampEntry, &Interaction, &mut Node)>,
    mut captions: Query<
        (
            &CampEntryCaption,
            &mut Text,
            &mut Node,
            &mut BackgroundColor,
        ),
        Without<CampEntry>,
    >,
) {
    let size = windows
        .iter()
        .next()
        .map_or(Vec2::new(1280.0, 720.0), |window| {
            Vec2::new(window.width(), window.height())
        });
    let docked = size.y < 540.0;
    let mut preview = [false; 3];
    for (entry, interaction, mut node) in &mut entries {
        preview[usize::from(entry.0)] = *interaction != Interaction::None;
        node.width = px(if docked { 40.0 } else { 128.0 });
        node.min_height = px(40.0);
        let position = cameras.single().ok().and_then(|(camera, t)| {
            camera
                .world_to_viewport(t, ADVENTURE_SITES[usize::from(entry.0)] + Vec3::Y * 1.7)
                .ok()
        });
        node.display = if !session.finished
            && state.pending.is_none()
            && state.active.is_none()
            && state.resolved & (1 << entry.0) == 0
            && position.is_some()
        {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(p) = position {
            let position = camp_screen_position(p, size, usize::from(entry.0));
            node.left = px(position.x);
            node.top = px(position.y);
        }
    }
    for (entry, mut text, mut node, mut background) in &mut captions {
        let index = usize::from(entry.0);
        node.display = if docked && !preview[index] {
            Display::None
        } else {
            Display::Flex
        };
        node.position_type = if docked {
            PositionType::Absolute
        } else {
            PositionType::Relative
        };
        node.right = if docked { px(46.0) } else { Val::Auto };
        node.top = if docked { px(0.0) } else { Val::Auto };
        node.width = if docked { px(148.0) } else { Val::Auto };
        node.padding = UiRect::all(px(if docked { 6.0 } else { 0.0 }));
        background.0 = if docked {
            Color::srgba(0.025, 0.04, 0.04, 0.96)
        } else {
            Color::NONE
        };
        let def = &ADVENTURE_BOSSES[index];
        let suffix = if state.unlocked & (1 << entry.0) != 0 {
            language.pick("Explore", "探索").to_string()
        } else {
            format!(
                "{} {}:{:02}",
                language.pick("Unlock", "解锁"),
                (ADVENTURE_OFFER_TIMES[index] / 60.0) as u32,
                (ADVENTURE_OFFER_TIMES[index] % 60.0) as u32
            )
        };
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(format!(
                "{}\n{suffix}",
                language.pick(def.name_en, def.name_zh)
            ));
    }
}

#[derive(Resource, Default)]
struct RelicToastState {
    owned: u8,
    root: Option<Entity>,
    remaining: f32,
}
fn relic_feedback(
    mut commands: Commands,
    mut state: ResMut<RelicToastState>,
    relics: Option<Res<super::basic_relics::BasicRelics>>,
    language: Res<GameLanguage>,
    time: Option<Res<Time<Real>>>,
) {
    let mask = relics.as_ref().map_or(0, |r| {
        (0..3).fold(0, |mask, i| mask | if r.owns(i) { 1 << i } else { 0 })
    });
    let acquired = mask & !state.owned;
    state.owned = mask;
    state.remaining -= time.as_ref().map_or(0.0, |t| t.delta_secs());
    if (state.remaining <= 0.0 || mask == 0 || acquired != 0)
        && let Some(root) = state.root.take()
    {
        commands.entity(root).try_despawn();
    }
    if acquired != 0 {
        let index = acquired.trailing_zeros() as u8;
        state.remaining = 3.0;
        state.root = Some(
            commands
                .spawn((
                    Name::new("Adventure relic acquired"),
                    Text::new(format!(
                        "{} · {}\n{}",
                        language.pick("Relic unlocked", "战利品已解锁"),
                        super::basic_relics::relic_name(index, *language),
                        super::basic_relics::relic_description(index, *language)
                    )),
                    TextFont {
                        font_size: FontSize::Px(13.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.97, 0.81, 0.4)),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(86),
                        left: percent(24),
                        width: percent(52),
                        padding: UiRect::all(px(10)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.025, 0.03, 0.035, 0.94)),
                    GlobalZIndex(290),
                ))
                .id(),
        );
    }
}

fn camp_screen_position(projected: Vec2, size: Vec2, index: usize) -> Vec2 {
    if size.y < 540.0 {
        return Vec2::new((size.x - 52.0).max(0.0), 132.0 + index as f32 * 48.0);
    }
    let top = 145.0;
    Vec2::new(
        (projected.x - 64.0).clamp(0.0, (size.x - 128.0).max(0.0)),
        projected.y.clamp(top, (size.y - 100.0).max(top)),
    )
}
#[cfg(test)]
mod camp_position_tests {
    use super::*;
    #[test]
    fn desktop_entry_clears_main_boss_health_title() {
        assert_eq!(
            camp_screen_position(Vec2::new(640.0, 70.0), Vec2::new(1280.0, 720.0), 0).y,
            145.0
        );
    }
    #[test]
    fn mobile_camp_pointer_target_clears_top_hud_and_stays_on_screen() {
        let p = camp_screen_position(Vec2::new(340.0, 31.0), Vec2::new(667.0, 375.0), 0);
        assert_eq!(p, Vec2::new(615.0, 132.0));
        assert!(p.y + 40.0 <= 275.0);
        let edge = camp_screen_position(Vec2::new(700.0, 600.0), Vec2::new(667.0, 375.0), 0);
        assert_eq!(edge, Vec2::new(615.0, 132.0));
    }
}
