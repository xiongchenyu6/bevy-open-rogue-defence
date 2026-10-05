//! A portrait opens the live combat attribute sheet; the sheet owns its pause.
#[path = "hero_fusion_ui.rs"]
mod fusion_ui;

use super::super::{
    GameLanguage, GameSession, HeroStatsSnapshot, SelectedHero, SettingsInputSet, content::HEROES,
    localization::hero_name,
};
use super::{HudHint, hud_metrics};
use bevy::{
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

#[derive(Resource, Default)]
pub(crate) struct HeroStatsPanelState {
    root: Option<Entity>,
    size: Vec2,
    language: Option<GameLanguage>,
}
impl HeroStatsPanelState {
    pub(crate) fn open(&self) -> bool {
        self.root.is_some()
    }
}
#[derive(Component)]
struct Open;
#[derive(Component)]
struct Close;
#[derive(Component)]
struct CloseHint;
#[derive(Component)]
struct Portrait;
#[derive(Component)]
struct ScrollBody;
#[derive(Component)]
struct StatValue(&'static str);
#[derive(Component)]
struct BasicAbilityText;

pub(super) fn install(app: &mut App) {
    app.init_resource::<HeroStatsPanelState>()
        .add_systems(
            PreUpdate,
            handle
                .after(bevy::ui::UiSystems::Focus)
                .after(SettingsInputSet),
        )
        .add_systems(
            Update,
            (update_portrait, update_values, update_close_hint, scroll),
        );
}

pub(super) fn spawn_entry(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Button,
            Open,
            Name::new("Hero attributes open"),
            HudHint::Attributes,
            Node {
                width: px(44),
                height: px(44),
                margin: UiRect::left(px(12)),
                border: UiRect::all(px(1)),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.035, 0.055, 0.065)),
            BorderColor::all(Color::srgb(0.81, 0.65, 0.36)),
        ))
        .with_children(|entry| {
            entry.spawn((
                Portrait,
                ImageNode::new(assets.load("ui/portraits/hero_storm_savant.png")),
                Node {
                    width: percent(100),
                    height: percent(100),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        });
}

fn update_portrait(
    selected: Res<SelectedHero>,
    assets: Res<AssetServer>,
    mut portraits: Query<&mut ImageNode, With<Portrait>>,
) {
    if !selected.is_changed() {
        return;
    }
    for mut portrait in &mut portraits {
        portrait.image = assets.load(format!(
            "ui/portraits/hero_{}.png",
            HEROES[selected.kind.index()].id
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn handle(
    mut commands: Commands,
    mut state: ResMut<HeroStatsPanelState>,
    mut session: ResMut<GameSession>,
    selected: Res<SelectedHero>,
    language: Res<GameLanguage>,
    snapshot: Res<HeroStatsSnapshot>,
    assets: Res<AssetServer>,
    fusion: Res<super::super::HeroFusion>,
    build: Res<super::super::PlayerBuild>,
    evolution: Res<super::super::skill_evolution::SkillEvolution>,
    windows: Query<&Window>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    open: Query<&Interaction, (With<Open>, Changed<Interaction>)>,
    close: Query<&Interaction, (With<Close>, Changed<Interaction>)>,
    settings: Res<super::super::settings::SettingsState>,
    equipment: Res<super::super::equipment_ui::EquipmentPanelState>,
) {
    if settings.open || equipment.open() {
        return;
    }
    let size = windows
        .single()
        .map(|w| Vec2::new(w.width(), w.height()))
        .unwrap_or(Vec2::new(1280., 720.));
    if state.open() {
        let closing = keyboard.just_pressed(KeyCode::Escape)
            || close.iter().any(|i| *i == Interaction::Pressed)
            || session.finished;
        keyboard.clear();
        if closing {
            commands.entity(state.root.take().unwrap()).despawn();
            session.paused = session.finished;
            return;
        }
        session.paused = true;
        if state.size == size && state.language == Some(*language) {
            return;
        }
        commands.entity(state.root.take().unwrap()).despawn();
    } else {
        if session.paused
            || session.finished
            || session.save_active
            || !selected.locked
            || !open.iter().any(|i| *i == Interaction::Pressed)
        {
            return;
        }
        session.paused = true;
        keyboard.clear();
    }
    state.size = size;
    state.language = Some(*language);
    state.root = Some(spawn_panel(
        &mut commands,
        &assets,
        &selected,
        *language,
        &snapshot,
        &fusion,
        &build,
        &evolution,
        size,
    ));
}

struct Row {
    field: &'static str,
    label: &'static str,
    value: String,
}
fn rows(language: GameLanguage, s: &HeroStatsSnapshot) -> [Vec<Row>; 3] {
    let row = |field, en, zh, value| Row {
        field,
        label: language.pick(en, zh),
        value,
    };
    let number = |value: f32| format!("{value:.1}");
    let percent_value = |value: f32| format!("{:.1}%", value * 100.0);
    [
        vec![
            row("strength", "Strength", "力量", number(s.strength)),
            row("agility", "Agility", "敏捷", number(s.agility)),
            row(
                "intelligence",
                "Intelligence",
                "智力",
                number(s.intelligence),
            ),
            row(
                "health",
                "Health",
                "生命",
                format!("{:.0}/{:.0}", s.health, s.max_health),
            ),
            row(
                "energy",
                "Energy",
                "能量",
                format!("{:.0}/{:.0}", s.energy, s.max_energy),
            ),
            row("move_speed", "Move speed", "移动速度", number(s.move_speed)),
            row(
                "spell_power",
                "Spell power",
                "法术强度",
                number(s.spell_power),
            ),
            row(
                "summon_power",
                "Summon power",
                "召唤强度",
                number(s.summon_power),
            ),
        ],
        vec![
            row(
                "attack_damage",
                "Base attack",
                "基础攻击",
                number(s.attack_damage),
            ),
            row(
                "basic_hit_damage",
                "Attack damage",
                "普攻伤害",
                if (s.basic_hit_max - s.basic_hit_min).abs() > 0.01 {
                    format!("{:.0}–{:.0}", s.basic_hit_min, s.basic_hit_max)
                } else {
                    number(s.basic_hit_damage)
                },
            ),
            row(
                "attacks_per_second",
                "Attacks/sec",
                "每秒攻击",
                format!("{:.2}", s.attacks_per_second),
            ),
            row(
                "attack_range",
                "Attack range",
                "攻击距离",
                number(s.attack_range),
            ),
            row(
                "crit_chance",
                "Critical chance",
                "暴击概率",
                percent_value(s.crit_chance),
            ),
            row(
                "crit_multiplier",
                "Critical damage",
                "暴击伤害",
                format!("×{:.2}", s.crit_multiplier),
            ),
            row(
                "life_leech",
                "Lifesteal",
                "伤害吸血",
                percent_value(s.life_leech),
            ),
            row(
                "physical_penetration",
                "Armor penetration",
                "物理穿透（点）",
                number(s.physical_penetration),
            ),
            row(
                "spell_penetration",
                "Magic penetration",
                "法术穿透",
                percent_value(s.spell_penetration),
            ),
        ],
        vec![
            row("armor", "Armor", "护甲", number(s.armor)),
            row(
                "physical_damage_reduction",
                "Physical reduction",
                "物理减伤",
                percent_value(s.physical_damage_reduction),
            ),
            row(
                "magic_resistance",
                "Magic resistance",
                "魔法抗性",
                percent_value(s.magic_resistance),
            ),
            row("evasion", "Evasion", "闪避概率", percent_value(s.evasion)),
            row(
                "summon_cap",
                "Summon limit",
                "召唤上限",
                s.summon_cap.to_string(),
            ),
        ],
    ]
}

fn growth_sources(language: GameLanguage) -> String {
    use super::super::market::MarketItem;
    let mut lines = vec![
        language
            .pick(
                "GROWTH SOURCES · Black Market equipment",
                "成长来源 · 黑市装备",
            )
            .to_owned(),
    ];
    for item in [
        MarketItem::HardenedPlate,
        MarketItem::WindBoots,
        MarketItem::BloodContract,
        MarketItem::SerratedDisc,
        MarketItem::ArcLens,
        MarketItem::EmberHeart,
    ] {
        lines.push(format!(
            "{} · {}",
            item.name(language),
            item.effect_text(language)
        ));
    }
    lines.push(language.pick("Equipment bonuses persist through hero absorption. Armor penetration ignores armor points; magic penetration reduces resistance by percentage points. Evasion applies to enemy attacks, excluding area explosions.", "装备加成在吞噬换英雄后保留。物穿无视相应点数的护甲；法穿降低敌方对应百分比的抗性。闪避针对敌方攻击，不含范围自爆。").to_owned());
    lines.join("\n")
}

fn spawn_panel(
    commands: &mut Commands,
    assets: &AssetServer,
    selected: &SelectedHero,
    language: GameLanguage,
    snapshot: &HeroStatsSnapshot,
    fusion: &super::super::HeroFusion,
    build: &super::super::PlayerBuild,
    evolution: &super::super::skill_evolution::SkillEvolution,
    size: Vec2,
) -> Entity {
    let compact = hud_metrics(size.x, size.y).compact;
    let stacked = size.x < 600.0;
    let font = if compact { 12.0 } else { 14.0 };
    let groups = rows(language, snapshot);
    commands.spawn((Name::new("Hero attributes overlay"),
        Node { position_type: PositionType::Absolute, width: percent(100), height: percent(100),
            align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
        BackgroundColor(Color::srgba(0.0, 0.01, 0.02, 0.78)), GlobalZIndex(280),
    )).with_children(|overlay| {
        overlay.spawn((Name::new("Hero attributes panel"),
            Node { width: px((size.x-24.0).min(900.0)), height: px((size.y-24.0).min(620.0)),
                padding: UiRect::all(px(12)), border: UiRect::all(px(1)),
                flex_direction: FlexDirection::Column, row_gap: px(10), ..default() },
            BackgroundColor(Color::srgb(0.028, 0.05, 0.06)), BorderColor::all(Color::srgb(0.8, 0.64, 0.35)),
        )).with_children(|panel| {
            panel.spawn(Node { min_height: px(44), align_items: AlignItems::Center, column_gap: px(12), flex_shrink: 0.0, ..default() }).with_children(|header| {
                header.spawn((ImageNode::new(assets.load(format!("ui/portraits/hero_{}.png", HEROES[selected.kind.index()].id))),
                    Node { width: px(44), height: px(44), ..default() }, Pickable::IGNORE));
                header.spawn((Text::new(format!("{} · {} {}", hero_name(language, selected.kind), language.pick("Level", "等级"), snapshot.level)),
                    TextFont { font_size: FontSize::Px(if compact {16.} else {21.}), ..default() },
                    TextColor(Color::srgb(0.96, 0.91, 0.78)), Node { flex_grow: 1.0, ..default() }));
                header.spawn((Button, Close, Name::new("Hero attributes close"),
                    Node { width: px(44), height: px(44), flex_shrink: 0.0, align_items: AlignItems::Center, justify_content: JustifyContent::Center, ..default() },
                    BackgroundColor(Color::srgb(0.17, 0.2, 0.22)),
                )).with_children(|button| {
                    button.spawn((Text::new("×"), TextFont { font_size: FontSize::Px(24.), ..default() }, TextColor(Color::WHITE), Pickable::IGNORE));
                    button.spawn((CloseHint, Text::new(language.pick("Close · Escape", "关闭 · Escape")),
                        TextFont {font_size: FontSize::Px(12.), ..default()}, TextColor(Color::WHITE),
                        Node {position_type: PositionType::Absolute, right: px(0), top: px(46), width: px(110), padding: UiRect::all(px(6)), display: Display::None, ..default()},
                        BackgroundColor(Color::srgb(0.08,0.11,0.13)), GlobalZIndex(282), Pickable::IGNORE));
                });
            });
            panel.spawn((ScrollBody, ScrollPosition::default(), Node {
                flex_grow: 1.0, min_height: px(0), overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(16), ..default()
            })).with_children(|body| {
                super::survival_ui::spawn(body, true);
                body.spawn((Name::new("Hero basic attack details"),BasicAbilityText,Text::new(crate::game::basic_attack_details::detail(language,selected.kind,snapshot.level,build,snapshot.basic_hit_damage,snapshot.cleave_target_bonus)),TextFont {font_size:FontSize::Px(if compact{11.0}else{13.0}),..default()},TextColor(Color::srgb(0.95,0.78,0.42)),Node{flex_shrink:0.0,..default()}));
                fusion_ui::spawn(body, assets, language, fusion, build, snapshot.level, snapshot.cleave_target_bonus, compact);
                for (index, path) in evolution.paths.iter().enumerate() {
                    if let Some(path) = path && build.skills[index] > 0 {
                        let skill = super::super::SkillId(index as u8);
                        body.spawn((Name::new(format!("UR evolution {index}")),Text::new(format!("{} · {}",super::super::localization::skill_name(language,skill),super::super::ur_evolution::name(language,skill,*path))),TextFont {font_size:FontSize::Px(if compact {12.0}else{15.0}),..default()},TextColor(Color::srgb(1.0,0.58,0.32)),Node {flex_shrink:0.0,..default()}));
                    }
                }
                body.spawn(Node { flex_direction: if stacked {FlexDirection::Column} else {FlexDirection::Row}, column_gap:px(16), row_gap:px(16), flex_shrink:0.0, ..default() }).with_children(|body| {
                for (index, group) in groups.into_iter().enumerate() {
                    body.spawn(Node { flex_direction: FlexDirection::Column,
                        flex_grow: if stacked {0.} else {1.}, flex_basis: if stacked {Val::Auto} else {px(0)},
                        flex_shrink: 0.0, min_width: px(0), row_gap: px(3), ..default() }).with_children(|column| {
                        column.spawn((Text::new([language.pick("ATTRIBUTES", "基础属性"), language.pick("OFFENSE", "攻击属性"), language.pick("DEFENSE", "防御属性")][index]),
                            TextFont {font_size: FontSize::Px(font), ..default()}, TextColor(Color::srgb(0.93,0.73,0.36)), Node { margin: UiRect::bottom(px(5)), ..default() }));
                        for row in group {
                            column.spawn(Node { width: percent(100), min_height: px(if compact {20.} else {27.}), align_items: AlignItems::Center, flex_shrink: 0.0, ..default() }).with_children(|cell| {
                                cell.spawn((Name::new(format!("Hero stat label {}", row.field)), Text::new(row.label), TextFont {font_size: FontSize::Px(font), ..default()}, TextColor(Color::srgb(0.66,0.74,0.76)), Node { width: percent(60), flex_shrink: 0.0, ..default() }));
                                cell.spawn((StatValue(row.field), Name::new(format!("Hero stat {}", row.field)), Text::new(row.value), TextFont {font_size: FontSize::Px(font), ..default()}, TextColor(Color::srgb(0.98,0.94,0.82)), TextLayout::justify(Justify::Right), Node {width: percent(40), flex_shrink: 0.0, ..default()}));
                            });
                        }
                    });
                }
                });
                body.spawn((Name::new("Hero attribute growth sources"), Text::new(growth_sources(language)), TextFont {font_size: FontSize::Px(if compact {11.0} else {13.0}), ..default()}, TextColor(Color::srgb(0.76,0.79,0.70)), Node {flex_shrink:0.0, ..default()}));
            });
            panel.spawn((Text::new(language.pick("Current bonuses included. Scroll for all attributes and growth sources. Lifesteal uses damage dealt.", "已计入当前加成。滚轮／拖动查看全部属性与成长来源；吸血按实际伤害计算。")),
                TextFont {font_size: FontSize::Px(if compact {11.} else {12.}), ..default()}, TextColor(Color::srgb(0.56,0.65,0.68)), Node {flex_shrink: 0.0, ..default()}));
        });
    }).id()
}

fn update_values(
    relics: Option<Res<crate::game::basic_relics::BasicRelics>>,
    snapshot: Res<HeroStatsSnapshot>,
    language: Res<GameLanguage>,
    selected: Res<SelectedHero>,
    build: Res<crate::game::PlayerBuild>,
    mut ability_text: Query<&mut Text, (With<BasicAbilityText>, Without<StatValue>)>,
    mut texts: Query<(&StatValue, &mut Text), Without<BasicAbilityText>>,
) {
    for mut text in &mut ability_text {
        **text = crate::game::basic_attack_details::detail(
            *language,
            selected.kind,
            snapshot.level,
            &build,
            snapshot.basic_hit_damage,
            snapshot.cleave_target_bonus,
        );
        if let Some(relics) = relics.as_ref() {
            for index in 0..3 {
                if relics.owns(index) {
                    text.push_str(&format!(
                        "\n{} · {}",
                        crate::game::basic_relics::relic_name(index, *language),
                        crate::game::basic_relics::relic_description(index, *language)
                    ));
                }
            }
        }
    }
    if texts.is_empty() {
        return;
    }
    let values = rows(*language, &snapshot);
    for (field, mut text) in &mut texts {
        if let Some(row) = values.iter().flatten().find(|row| row.field == field.0) {
            **text = row.value.clone();
        }
    }
}
fn scroll(
    mut wheel: MessageReader<MouseWheel>,
    touches: Res<Touches>,
    mut bodies: Query<&mut ScrollPosition, With<ScrollBody>>,
) {
    let delta: f32 = wheel
        .read()
        .map(|w| {
            -w.y * if w.unit == MouseScrollUnit::Line {
                24.
            } else {
                1.
            }
        })
        .sum();
    let drag = touches.iter().map(|touch| -touch.delta().y).sum::<f32>();
    for mut position in &mut bodies {
        position.0.y = (position.0.y + delta + drag).max(0.);
    }
}

fn update_close_hint(
    buttons: Query<&Interaction, With<Close>>,
    mut hints: Query<&mut Node, With<CloseHint>>,
) {
    let show = buttons
        .iter()
        .any(|interaction| *interaction != Interaction::None);
    for mut node in &mut hints {
        node.display = if show { Display::Flex } else { Display::None };
    }
}

pub(super) fn reset_run(world: &mut World) {
    world.insert_resource(HeroStatsPanelState::default());
}
