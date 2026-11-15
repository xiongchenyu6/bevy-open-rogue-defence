//! Chapter selection, persistent training, and actionable run results.
use super::{
    GameLanguage, GameSession, RunSeed,
    campaign::{CampaignRun, RunRequest, chapter},
    campaign_progress::{CampaignProgress, CampaignReward, MAX_UPGRADE_LEVEL, UPGRADE_PER_LEVEL},
};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
const GOLD: Color = Color::srgb(0.92, 0.73, 0.39);
const IVORY: Color = Color::srgb(0.96, 0.93, 0.84);
const MUTED: Color = Color::srgb(0.60, 0.70, 0.72);
#[derive(Resource)]
pub struct CampaignMenu(pub bool);
impl Default for CampaignMenu {
    fn default() -> Self {
        Self(true)
    }
}
#[derive(Component)]
pub(crate) struct CampaignOverlay;
#[derive(Component)]
struct ScrollBody;
#[derive(Component)]
struct Hint(String);
#[derive(Component)]
struct HintText(String);
#[derive(Component, Clone, Copy)]
enum Action {
    Start(usize),
    Chapters,
    Training,
    Language,
    Buy(usize),
    RetrySave,
}
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Page {
    #[default]
    Chapters,
    Training,
    Result,
}
#[derive(Resource, Default)]
struct MenuView {
    root: Option<Entity>,
    page: Page,
    drawn: Option<Page>,
    size: Vec2,
    language: Option<GameLanguage>,
    dirty: bool,
    notice: String,
}

pub(crate) struct CampaignUiPlugin;
impl Plugin for CampaignUiPlugin {
    fn build(&self, app: &mut App) {
        install(app);
    }
}
fn install(app: &mut App) {
    app.init_resource::<CampaignMenu>()
        .init_resource::<MenuView>()
        .add_systems(
            Update,
            (input, render, scroll, tooltips, controls::feedback).chain(),
        );
}
fn label(text: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}
mod controls;
use controls::{button, toolbar_button};

#[allow(clippy::too_many_arguments)]
fn input(
    mut menu: ResMut<CampaignMenu>,
    mut view: ResMut<MenuView>,
    mut request: ResMut<RunRequest>,
    mut session: ResMut<GameSession>,
    mut progress: ResMut<CampaignProgress>,
    mut language: ResMut<GameLanguage>,
    buttons: Query<(&Action, &Interaction), Changed<Interaction>>,
) {
    for (action, interaction) in &buttons {
        if *interaction != Interaction::Pressed {
            continue;
        }
        match *action {
            Action::Start(stage) if progress.can_play(stage) => {
                request.0 = Some(stage);
                menu.0 = false;
                view.notice.clear();
            }
            Action::Start(_) => {}
            Action::Chapters => {
                menu.0 = true;
                view.page = Page::Chapters;
                view.notice.clear();
                session.paused = true;
            }
            Action::Training => {
                menu.0 = true;
                view.page = Page::Training;
                view.notice.clear();
                session.paused = true;
            }
            Action::Language => {
                *language = if language.is_chinese() {
                    GameLanguage::English
                } else {
                    GameLanguage::SimplifiedChinese
                };
                if let Err(error) = crate::preferences::save_language(*language) {
                    view.notice = format!(
                        "{}: {error}",
                        language.pick("Language could not be saved", "语言未能保存")
                    );
                }
            }
            Action::RetrySave => {
                view.notice = match progress.retry_save() {
                    Ok(()) => language.pick("Progress saved.", "进度已保存。").into(),
                    Err(_) => language
                        .pick(
                            "Saving failed. Keep the game open and retry saving.",
                            "保存失败。请保持游戏开启并重试保存。",
                        )
                        .into(),
                };
            }
            Action::Buy(index) => {
                if progress.purchase(index) {
                    view.notice=match progress.persist() {Ok(())=>language.pick("Saved · Applies to your next expedition.","已保存 · 下次远征生效。").into(),Err(_)=>language.pick("Upgrade purchased, but saving failed. Keep the game open and try again.","强化已购买，但保存失败。请保持游戏开启并重试。").into()};
                } else {
                    view.notice = language
                        .pick(
                            "Not enough shards, or this upgrade is complete.",
                            "碎片不足，或该强化已满级。",
                        )
                        .into();
                }
            }
        }
        view.dirty = true;
    }
}
fn stage_title(language: GameLanguage, stage: usize) -> &'static str {
    let titles = [
        ("The Broken Gate", "残垣之门"),
        ("Ember March", "余烬行军"),
        ("Frozen Crossing", "霜冻渡口"),
        ("Hollow Grove", "幽影林地"),
        ("Siege of Dawn", "黎明围城"),
        ("The Last Rift", "终焉裂隙"),
    ];
    let (en, zh) = titles[stage.min(5)];
    language.pick(en, zh)
}
fn stage_icon(stage: usize) -> &'static str {
    [
        "tower",
        "ember_rebirth",
        "glacier_core",
        "spirit_pact",
        "guardian_oath",
        "execution_brand",
    ][stage.min(5)]
}
fn chapter_hint(language: GameLanguage, stage: usize, unlocked: bool) -> String {
    if !unlocked {
        return language
            .pick(
                "Clear the previous chapter to unlock this expedition.",
                "通关前一章节后解锁这场远征。",
            )
            .into();
    }
    let rules = chapter(stage);
    if language.is_chinese() {
        format!(
            "守住25波敌人与5名首领 · 生命×{:.2} / 伤害×{:.2} · 首通额外{}碎片",
            rules.health_scale,
            rules.damage_scale,
            40 + stage * 10
        )
    } else {
        format!(
            "Defend against 25 waves and 5 bosses · HP ×{:.2} / damage ×{:.2} · First clear +{} shards",
            rules.health_scale,
            rules.damage_scale,
            40 + stage * 10
        )
    }
}
#[allow(clippy::too_many_arguments)]
fn render(
    mut commands: Commands,
    menu: Res<CampaignMenu>,
    mut view: ResMut<MenuView>,
    mut session: ResMut<GameSession>,
    run: Res<CampaignRun>,
    progress: Res<CampaignProgress>,
    reward: Res<CampaignReward>,
    language: Res<GameLanguage>,
    seed: Res<RunSeed>,
    assets: Res<AssetServer>,
    windows: Query<&Window>,
    roots: Query<(), With<CampaignOverlay>>,
) {
    let page = if menu.0 {
        Some(view.page)
    } else if session.finished {
        Some(Page::Result)
    } else {
        None
    };
    let Some(page) = page else {
        if let Some(root) = view.root.take()
            && roots.contains(root)
        {
            commands.entity(root).despawn();
        }
        view.drawn = None;
        return;
    };
    session.paused = true;
    // A result can be revisited through Training without losing the settled reward.
    let size = windows
        .iter()
        .next()
        .map(|window| Vec2::new(window.width(), window.height()))
        .unwrap_or(Vec2::new(1280.0, 720.0));
    if view.root.is_some_and(|root| roots.contains(root))
        && view.drawn == Some(page)
        && view.size == size
        && view.language == Some(*language)
        && !view.dirty
        && !progress.is_changed()
        && !reward.is_changed()
    {
        return;
    }
    if let Some(root) = view.root.take()
        && roots.contains(root)
    {
        commands.entity(root).despawn();
    }
    let compact = size.y < 640.0 || size.x < 480.0;
    let width = (size.x - 24.0).min(1080.0);
    let height = (size.y - 24.0).min(720.0);
    let root=commands.spawn((Name::new("Campaign overlay"),CampaignOverlay,Node {position_type:PositionType::Absolute,width:percent(100),height:percent(100),align_items:AlignItems::Center,justify_content:JustifyContent::Center,..default()},BackgroundColor(Color::srgb(0.012,0.026,0.031)),GlobalZIndex(400), crate::game::controller_input::ControllerScope(400))).with_children(|overlay| {
        overlay.spawn((Name::new("Campaign panel"),Node {width:px(width),height:px(height),padding:UiRect::all(px(if compact {10}else{20})),flex_direction:FlexDirection::Column,row_gap:px(if compact {8}else{14}),..default()},BackgroundColor(Color::srgb(0.025,0.05,0.058)))).with_children(|panel| {
            panel.spawn(Node {align_items:AlignItems::Center,column_gap:px(8),flex_shrink:0.0,..default()}).with_children(|header| {
                header.spawn((Name::new("Campaign heading"),label(match page {Page::Chapters=>language.pick("EXPEDITIONS","章节远征"),Page::Training=>language.pick("LEGACY TRAINING","永久强化"),Page::Result=>language.pick("EXPEDITION REPORT","远征战报")},if compact {21.0}else{30.0},IVORY),Node {flex_grow:1.0,min_width:px(0),..default()}));
                toolbar_button(header,&assets,"Campaign language",language.pick("Change language", "切换语言"),Action::Language,Some(language.pick("EN", "中")));
                if page==Page::Chapters {toolbar_button(header,&assets,"Campaign training",language.pick("Permanent training","永久强化"),Action::Training,None);}else if page==Page::Training {toolbar_button(header,&assets,"Campaign chapters",language.pick("Choose chapter","章节选择"),Action::Chapters,None);}
            });
            panel.spawn(Node { column_gap:px(6), align_items:AlignItems::Center, flex_shrink:0.0, ..default() }).with_children(|currency| {
                currency.spawn((ImageNode::new(assets.load("ui/icons/fate_cube.png")),Node {width:px(20),height:px(20),..default()},Pickable::IGNORE));
                currency.spawn(label(format!("{}",progress.currency),14.0,GOLD));
            });
            panel.spawn((Name::new("Campaign scroll body"),ScrollBody,ScrollPosition::default(),Node {flex_grow:1.0,min_height:px(0),overflow:Overflow::scroll_y(),flex_direction:FlexDirection::Column,..default()})).with_children(|body| {
                match page {
                    Page::Chapters=>chapters(body,&assets,*language,&progress,width,height,compact),
                    Page::Training=>training(body,&assets,*language,&progress,width,compact),
                    Page::Result=>result(body,&assets,*language,&session,&run,&reward,seed.0,compact),
                }
            });
            if page==Page::Result {result_actions(panel,&assets,*language,&session,&run,&progress);}
            if page==Page::Training&&session.finished {panel.spawn(Node {column_gap:px(8),flex_wrap:FlexWrap::Wrap,..default()}).with_children(|footer| {button(footer,&assets,"Campaign retry",language.pick("Retry chapter","重试本关"),Action::Start(run.stage),true);});}
            if progress.can_retry_save() { button(panel,&assets,"Campaign retry save",language.pick("Retry save","重试保存"),Action::RetrySave,true); }
            let footer_copy = if progress.can_retry_save() { language.pick("Saving failed. Your rewards and upgrades are kept while the game stays open. Retry saving.","保存失败。保持游戏开启可保留奖励和强化，请重试保存。") } else if progress.last_error.is_some() { language.pick("Progress could not be loaded. This session is temporary and cannot be saved. Existing save data was preserved.","进度读取失败。本局为临时模式，无法保存。原存档已保留。") } else if view.notice.is_empty() {match page {Page::Chapters=>language.pick("Select a chapter. Every expedition starts with a new commander.","选择章节，每次远征从新的基础指挥官开始。"),Page::Training=>language.pick("Permanent bonuses apply to every new expedition. Scroll for more upgrades.","永久加成应用于每次新远征，可滚动查看全部强化。"),Page::Result=>language.pick("Your shards and unlocked chapters are saved between expeditions.","碎片与已解锁章节将在远征之间保留。")}}else{&view.notice}.to_string();
            panel.spawn((Name::new("Campaign hint"),HintText(footer_copy.clone()),label(footer_copy,if compact {10.0}else{12.0},MUTED),Node {min_height:px(if compact {25}else{32}),flex_shrink:0.0,..default()}));
        });
    }).id();
    view.root = Some(root);
    view.drawn = Some(page);
    view.size = size;
    view.language = Some(*language);
    view.dirty = false;
}
fn chapter_card_height(panel_height: f32, columns: f32, compact: bool) -> f32 {
    let rows = (6.0 / columns).ceil();
    let chrome = if compact { 142.0 } else { 190.0 };
    ((panel_height - chrome - (rows - 1.0) * 10.0) / rows).clamp(94.0, 205.0)
}

fn chapters(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    language: GameLanguage,
    progress: &CampaignProgress,
    width: f32,
    height: f32,
    compact: bool,
) {
    let columns = if width < 480.0 { 2.0 } else { 3.0 };
    let card_width =
        (width - if compact { 20.0 } else { 40.0 } - (columns - 1.0) * 10.0 - 2.0) / columns;
    // Reserve header, currency, hint, gaps and padding before sizing the grid.
    // Short landscape screens must show both rows without relying on hidden scrolling.
    let narrow_card = card_width < 160.0;
    // Narrow portrait cards can wrap English titles onto a second line. Preserve
    // readable content at tiny sizes and let ScrollBody handle the extra rows.
    let card_height =
        chapter_card_height(height, columns, compact).max(if narrow_card { 112.0 } else { 94.0 });
    let small_card = card_height < 145.0;
    let icon_size = (card_height - if narrow_card { 112.0 } else { 88.0 }).clamp(32.0, 96.0);
    parent
        .spawn(Node {
            flex_shrink: 0.0,
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(10),
            row_gap: px(10),
            align_content: AlignContent::Start,
            ..default()
        })
        .with_children(|grid| {
            for stage in 0..6 {
                let unlocked = progress.can_play(stage);
                grid.spawn((
                    Name::new(format!("Campaign chapter {stage}")),
                    Button,
                    Action::Start(stage),
                    Hint(chapter_hint(language, stage, unlocked)),
                    Node {
                        width: px(card_width),
                        height: px(card_height),
                        border_radius: BorderRadius::all(px(10)),
                        padding: UiRect::all(px(if small_card { 5 } else { 10 })),
                        border: UiRect::all(px(1)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        row_gap: px(if small_card { 2 } else { 5 }),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(if unlocked {
                        Color::srgb(0.055, 0.105, 0.12)
                    } else {
                        Color::srgb(0.035, 0.047, 0.052)
                    }),
                    BorderColor::all(if unlocked {
                        GOLD
                    } else {
                        Color::srgb(0.19, 0.30, 0.32)
                    }),
                ))
                .insert_if(super::controller_input::ControllerDisabled, || !unlocked)
                .with_children(|card| {
                    card.spawn((
                        ImageNode {
                            image: assets.load(format!("ui/icons/{}.png", stage_icon(stage))),
                            color: Color::srgba(1.0, 1.0, 1.0, if unlocked { 1.0 } else { 0.28 }),
                            ..default()
                        },
                        Node {
                            width: px(icon_size),
                            height: px(icon_size),
                            flex_shrink: 0.0,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    card.spawn((
                        label(
                            format!("{:02}  {}", stage + 1, stage_title(language, stage)),
                            if compact { 12.0 } else { 17.0 },
                            if unlocked { IVORY } else { MUTED },
                        ),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ));
                    card.spawn((
                        label(
                            format!(
                                "{} {} · {} {:.1}×",
                                (chapter(stage).duration / 60.0).ceil() as u32,
                                language.pick("min", "分钟"),
                                language.pick("Enemy HP", "敌人生命"),
                                chapter(stage).health_scale
                            ),
                            10.0,
                            MUTED,
                        ),
                        Pickable::IGNORE,
                    ));
                    card.spawn((
                        ImageNode::new(assets.load(if progress.cleared[stage] {
                            "ui/icons/confirm.png"
                        } else if unlocked {
                            "ui/controls/play.png"
                        } else {
                            "ui/controls/lock.png"
                        })),
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(7),
                            right: px(7),
                            width: px(18),
                            height: px(18),
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                });
            }
        });
}
fn training(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    language: GameLanguage,
    progress: &CampaignProgress,
    width: f32,
    compact: bool,
) {
    let names = [
        ("Strength", "力量", "ground_pulse"),
        ("Agility", "敏捷", "dodge"),
        ("Intelligence", "智力", "arcane_star"),
        ("Health", "生命", "shield"),
        ("Attack", "攻击", "ember_arrow"),
        ("Spell power", "施法强度", "storm_matrix"),
    ];
    let columns = if width < 480.0 {
        1.0
    } else if compact {
        2.0
    } else {
        3.0
    };
    let card_width =
        (width - if compact { 20.0 } else { 40.0 } - (columns - 1.0) * 10.0 - 2.0) / columns;
    parent
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(10),
            row_gap: px(10),
            align_content: AlignContent::Start,
            ..default()
        })
        .with_children(|grid| {
            for (index, (en, zh, icon)) in names.into_iter().enumerate() {
                let level = progress.upgrades[index];
                let value = UPGRADE_PER_LEVEL[index] * f32::from(level);
                grid.spawn((
                    Name::new(format!("Campaign upgrade card {index}")),
                    Node {
                        width: px(card_width),
                        min_height: px(if compact { 145 } else { 190 }),
                        padding: UiRect::all(px(10)),
                        border: UiRect::all(px(1)),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(8),
                        flex_shrink: 0.0,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.09, 0.105)),
                    BorderColor::all(Color::srgb(0.2, 0.31, 0.33)),
                ))
                .with_children(|card| {
                    card.spawn(Node {
                        column_gap: px(8),
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|line| {
                        line.spawn((
                            ImageNode::new(assets.load(format!("ui/icons/{icon}.png"))),
                            Node {
                                width: px(40),
                                height: px(40),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                        line.spawn(label(
                            format!("{}\n{level}/{MAX_UPGRADE_LEVEL}", language.pick(en, zh)),
                            14.0,
                            IVORY,
                        ));
                    });
                    card.spawn(label(
                        if level < MAX_UPGRADE_LEVEL {
                            format!("+{value:.0}  →  +{:.0}", value + UPGRADE_PER_LEVEL[index])
                        } else {
                            format!("+{value:.0}")
                        },
                        20.0,
                        GOLD,
                    ));
                    let copy = progress
                        .upgrade_cost(index)
                        .map(|cost| format!("{}  {cost}", language.pick("Upgrade ·", "强化 ·")))
                        .unwrap_or_else(|| language.pick("MAXIMUM", "已满级").into());
                    button(
                        card,
                        assets,
                        format!("Campaign buy {index}"),
                        &copy,
                        Action::Buy(index),
                        progress
                            .upgrade_cost(index)
                            .is_some_and(|cost| progress.currency >= cost),
                    );
                });
            }
        });
}
#[allow(clippy::too_many_arguments)]
fn result(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    language: GameLanguage,
    session: &GameSession,
    run: &CampaignRun,
    reward: &CampaignReward,
    seed: u64,
    compact: bool,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(if compact { 9 } else { 18 }),
            padding: UiRect::all(px(8)),
            ..default()
        })
        .with_children(|report| {
            report.spawn((
                ImageNode::new(assets.load(format!(
                    "ui/icons/{}.png",
                    if session.victory {
                        "guardian_oath"
                    } else {
                        "skull"
                    }
                ))),
                Node {
                    width: px(if compact { 48 } else { 88 }),
                    height: px(if compact { 48 } else { 88 }),
                    ..default()
                },
                Pickable::IGNORE,
            ));
            report.spawn((
                label(
                    language.pick(
                        if session.victory {
                            "CHAPTER CLEARED"
                        } else {
                            "NO LIVES REMAIN"
                        },
                        if session.victory {
                            "章节已通关"
                        } else {
                            "英雄生命已耗尽"
                        },
                    ),
                    if compact { 24.0 } else { 38.0 },
                    if session.victory {
                        GOLD
                    } else {
                        Color::srgb(1.0, 0.53, 0.43)
                    },
                ),
                TextLayout::justify(Justify::Center),
            ));
            report.spawn(label(stage_title(language, run.stage), 16.0, IVORY));
            if !session.victory {
                report.spawn((label(language.pick("Your commander lost the last life. Retry or train for another expedition.", "英雄最后一次生命耗尽。可重新挑战，或强化后再出发。"), 13.0, IVORY), TextLayout::justify(Justify::Center)));
            }
            if let Some(gain) = &reward.last {
                report.spawn((
                    Name::new("Campaign reward total"),
                    label(
                        format!("+{} {}", gain.reward, language.pick("SHARDS", "碎片")),
                        if compact { 25.0 } else { 36.0 },
                        GOLD,
                    ),
                ));
                if gain.first_clear_bonus > 0 {
                    report.spawn(label(
                        format!(
                            "{} +{}",
                            language.pick("First clear bonus", "首通额外奖励"),
                            gain.first_clear_bonus
                        ),
                        13.0,
                        IVORY,
                    ));
                    if run.stage + 1 < 6 {
                        report.spawn(label(
                            format!(
                                "{}: {}",
                                language.pick("Unlocked", "已解锁"),
                                stage_title(language, run.stage + 1)
                            ),
                            13.0,
                            GOLD,
                        ));
                    }
                }
                if gain.imprint_bonus > 0 {
                    report.spawn((
                        Name::new("Campaign imprint bonus"),
                        label(imprint_bonus_text(language, gain.imprint_bonus), 13.0, IVORY),
                    ));
                }
            }
            report.spawn(label(
                if language.is_chinese() {
                    format!("等级 {} · 击杀 {}", session.level, session.total_kills)
                } else {
                    format!("Level {} · {} kills", session.level, session.total_kills)
                },
                13.0,
                MUTED,
            ));
            report.spawn((
                Name::new("Campaign result seed"),
                label(seed_text(language, seed), 10.0, MUTED),
            ));
        });
}
fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    mut bodies: Query<&mut ScrollPosition, With<ScrollBody>>,
) {
    let delta = wheel
        .read()
        .map(|event| {
            -event.y
                * if event.unit == MouseScrollUnit::Line {
                    28.0
                } else {
                    1.0
                }
        })
        .sum::<f32>()
        + touches.iter().map(|touch| -touch.delta().y).sum::<f32>();
    for mut position in &mut bodies {
        position.0.y = (position.0.y + delta).max(0.0);
    }
}
fn tooltips(
    hints: Query<(
        &Hint,
        &Interaction,
        Has<super::controller_input::ControllerFocused>,
    )>,
    mut texts: Query<(&HintText, &mut Text)>,
) {
    let hovered = hints
        .iter()
        .find(|(_, interaction, focused)| *focused || **interaction != Interaction::None)
        .map(|(hint, _, _)| hint.0.as_str());
    for (default, mut text) in &mut texts {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(hovered.unwrap_or(&default.0).to_string());
    }
}

fn imprint_bonus_text(language: GameLanguage, shards: u32) -> String {
    format!(
        "{} +{shards}",
        language.pick("Imprint shattered", "击破刻印")
    )
}

fn seed_text(language: GameLanguage, seed: u64) -> String {
    format!("{} 0x{seed:016X}", language.pick("SEED", "种子"))
}

fn result_actions(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    language: GameLanguage,
    session: &GameSession,
    run: &CampaignRun,
    progress: &CampaignProgress,
) {
    parent
        .spawn((
            Name::new("Campaign result actions"),
            Node {
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
                row_gap: px(8),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .with_children(|actions| {
            if session.victory && run.stage + 1 < 6 && progress.can_play(run.stage + 1) {
                button(
                    actions,
                    assets,
                    "Campaign next",
                    language.pick("Next chapter", "下一章节"),
                    Action::Start(run.stage + 1),
                    true,
                );
            }
            button(
                actions,
                assets,
                "Campaign retry",
                language.pick("Retry chapter", "重试本关"),
                Action::Start(run.stage),
                !session.victory,
            );
            button(
                actions,
                assets,
                "Campaign training",
                language.pick("Train & return", "强化再战"),
                Action::Training,
                false,
            );
            button(
                actions,
                assets,
                "Campaign chapters",
                language.pick("Chapters", "章节选择"),
                Action::Chapters,
                false,
            );
        });
}

/// Discard the current run before presenting an unambiguous chapter page.
pub(crate) fn open_chapters(world: &mut World) {
    let stage = world.resource::<CampaignRun>().stage;
    super::run_reset::reset_run(world, stage);
    let root = world.resource_mut::<MenuView>().root.take();
    if let Some(root) = root {
        let _ = world.despawn(root);
    }
    world.insert_resource(MenuView::default());
    world.resource_mut::<CampaignMenu>().0 = true;
    world.resource_mut::<GameSession>().paused = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> App {
        let mut app = App::new();
        app.init_resource::<CampaignMenu>()
            .init_resource::<MenuView>()
            .init_resource::<RunRequest>()
            .init_resource::<GameSession>()
            .init_resource::<CampaignProgress>()
            .init_resource::<GameLanguage>()
            .add_systems(Update, input);
        app
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn retry_save_button_persists_rewards_and_upgrade_without_repeating_either() {
        use super::super::ArchiveOutcome;
        let dir =
            std::env::temp_dir().join(format!("campaign-retry-button-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.save");
        let mut progress = CampaignProgress::load_from_path(&path).unwrap();
        progress.persist().unwrap();
        let original = std::fs::read(&path).unwrap();
        let backup = dir.with_extension("backup");
        let _ = std::fs::remove_dir_all(&backup);
        std::fs::rename(&dir, &backup).unwrap();
        std::fs::write(&dir, "temporarily blocked directory").unwrap();
        let mut reward = CampaignReward::default();
        reward
            .settle(&mut progress, 0, true, 200, ArchiveOutcome::TimedOut)
            .unwrap();
        assert!(progress.purchase(0));
        assert!(progress.persist().is_err());
        assert!(progress.can_retry_save());
        assert_eq!(
            std::fs::read(backup.join("progress.save")).unwrap(),
            original
        );
        let currency = progress.currency;
        let upgrades = progress.upgrades;
        let mut app = app();
        app.insert_resource(progress);
        let button = app
            .world_mut()
            .spawn((Action::RetrySave, Interaction::Pressed))
            .id();
        app.update();
        assert!(app.world().resource::<CampaignProgress>().can_retry_save());
        std::fs::remove_file(&dir).unwrap();
        std::fs::rename(&backup, &dir).unwrap();
        app.world_mut().entity_mut(button).insert(Interaction::None);
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Pressed);
        app.update();
        {
            let progress = app.world().resource::<CampaignProgress>();
            assert_eq!(progress.currency, currency);
            assert_eq!(progress.upgrades, upgrades);
            assert!(progress.last_error.is_none());
        }
        assert!(
            reward
                .settle(
                    &mut app.world_mut().resource_mut::<CampaignProgress>(),
                    0,
                    true,
                    200,
                    ArchiveOutcome::TimedOut
                )
                .is_none()
        );
        let saved = CampaignProgress::load_from_path(&path).unwrap();
        assert_eq!(saved.currency, currency);
        assert_eq!(saved.upgrades, upgrades);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn retry_button_cannot_overwrite_an_unread_save_even_after_it_is_repaired() {
        let dir =
            std::env::temp_dir().join(format!("campaign-unread-button-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("progress.save");
        std::fs::write(&path, "broken save").unwrap();
        let error = CampaignProgress::load_from_path(&path).unwrap_err();
        let mut app = app();
        app.world_mut()
            .resource_mut::<CampaignProgress>()
            .last_error = Some(error);
        let button = app
            .world_mut()
            .spawn((Action::RetrySave, Interaction::Pressed))
            .id();
        app.update();
        assert!(!app.world().resource::<CampaignProgress>().can_retry_save());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "broken save");
        std::fs::remove_file(&path).unwrap();
        let mut repaired = CampaignProgress::load_from_path(&path).unwrap();
        repaired.currency = 123;
        repaired.persist().unwrap();
        let original = std::fs::read(&path).unwrap();
        app.world_mut().entity_mut(button).insert(Interaction::None);
        app.update();
        app.world_mut()
            .entity_mut(button)
            .insert(Interaction::Pressed);
        app.update();
        assert_eq!(std::fs::read(&path).unwrap(), original);
        assert!(
            app.world()
                .resource::<CampaignProgress>()
                .last_error
                .is_some()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn result_retry_and_chapter_controls_issue_real_requests_without_spending_shards() {
        let mut app = app();
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.world_mut().resource_mut::<CampaignProgress>().currency = 25;
        app.world_mut()
            .spawn((Action::Start(0), Interaction::Pressed));
        app.update();
        assert_eq!(app.world().resource::<RunRequest>().0, Some(0));
        assert!(!app.world().resource::<CampaignMenu>().0);
        assert_eq!(app.world().resource::<CampaignProgress>().currency, 25);
    }
    #[test]
    fn locked_chapters_reject_clicks_and_training_purchases_once_per_press() {
        let mut app = app();
        app.world_mut().resource_mut::<CampaignProgress>().currency = 100;
        let locked = app
            .world_mut()
            .spawn((Action::Start(5), Interaction::Pressed))
            .id();
        app.update();
        assert_eq!(app.world().resource::<RunRequest>().0, None);
        app.world_mut().despawn(locked);
        app.world_mut()
            .spawn((Action::Buy(0), Interaction::Pressed));
        app.update();
        app.update();
        let progress = app.world().resource::<CampaignProgress>();
        assert_eq!(progress.upgrades[0], 1);
        assert_eq!(progress.currency, 85);
    }
    #[test]
    fn imprint_bonus_line_uses_only_the_selected_language() {
        assert_eq!(
            imprint_bonus_text(GameLanguage::English, 15),
            "Imprint shattered +15"
        );
        assert_eq!(
            imprint_bonus_text(GameLanguage::SimplifiedChinese, 40),
            "击破刻印 +40"
        );
    }
    #[test]
    fn result_seed_preserves_all_sixty_four_bits() {
        assert_eq!(
            seed_text(GameLanguage::English, 0x1234_5678_9ABC_DEF0),
            "SEED 0x123456789ABCDEF0"
        );
    }
}
