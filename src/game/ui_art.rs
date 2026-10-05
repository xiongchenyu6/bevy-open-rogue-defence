//! Icon counters, vitals gauges and contextual battle control hints.
use super::super::localization::bond_family_detail;
use super::super::{BondId, PlayerBuild};
use super::*;

#[derive(Component, Clone, Copy)]
pub(super) enum HudValue {
    Gold,
    Wood,
    Kills,
    Health,
    Energy,
}

#[derive(Component)]
pub(super) struct HudGauge(pub(super) HudValue);

#[derive(Component, Clone, Copy)]
pub(super) enum HudHint {
    Fruit,
    Bond,
    BondProgress(BondId),
    Encounter,
    Skill,
    Dodge,
    Attributes,
    Lives,
    Gate,
    Collection,
    Trials,
    Pause,
    Trial(TrialAction),
    Speed(SpeedAction),
}

#[derive(Component)]
pub(super) struct HudTooltip;

#[derive(Component)]
pub(super) struct HudTooltipText;

pub(super) fn hud_icon(
    parent: &mut ChildSpawnerCommands,
    assets: &AssetServer,
    name: &str,
    size: f32,
) {
    parent.spawn((
        ImageNode::new(assets.load(format!("ui/icons/{name}.png"))),
        Node {
            width: px(size),
            height: px(size),
            flex_shrink: 0.0,
            ..default()
        },
        Pickable::IGNORE,
    ));
}

#[derive(Default)]
pub(super) struct BondReview {
    pinned: Option<BondId>,
    dismissed: Option<BondId>,
}

impl BondReview {
    fn press(&mut self, badge: Option<BondId>) {
        self.dismissed = badge.filter(|id| self.pinned == Some(*id));
        self.pinned = badge.filter(|id| self.pinned != Some(*id));
    }
}

#[allow(clippy::type_complexity)]
pub(super) fn update_icon_hud(
    session: Res<GameSession>,
    vitals: Res<HeroVitals>,
    economy: Res<SkillEconomy>,
    encounters: Res<EncounterState>,
    build: Res<PlayerBuild>,
    bonds: Option<Res<super::super::bonds::BondRuntime>>,
    language: Res<GameLanguage>,
    hints: Query<(Ref<Interaction>, &HudHint)>,
    mut review: Local<BondReview>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    touches: Option<Res<Touches>>,
    mut values: Query<(&HudValue, &mut Text), Without<HudTooltipText>>,
    mut gauges: Query<(&HudGauge, &mut Node), Without<HudTooltip>>,
    mut tips: Query<&mut Node, (With<HudTooltip>, Without<HudGauge>)>,
    mut tip_text: Query<&mut Text, (With<HudTooltipText>, Without<HudValue>)>,
) {
    for (value, mut text) in &mut values {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(match value {
                HudValue::Gold => session.gold.to_string(),
                HudValue::Wood => session.wood.to_string(),
                HudValue::Kills => session.kills.to_string(),
                HudValue::Health if vitals.lives_remaining() == 0 => {
                    language.pick("NO LIVES LEFT", "生命已耗尽").into()
                }
                HudValue::Health if !vitals.alive => format!(
                    "{} {:.1}{}",
                    language.pick("Revive", "复活"),
                    vitals.revive_remaining,
                    language.pick("s", "秒")
                ),
                HudValue::Health => format!("{:.0} / {:.0}", vitals.health, vitals.max_health),
                HudValue::Energy => format!("{:.0} / {:.0}", vitals.energy, vitals.max_energy),
            });
    }
    for (gauge, mut node) in &mut gauges {
        let fraction = match gauge.0 {
            HudValue::Health => vitals.health / vitals.max_health.max(1.0),
            HudValue::Energy => vitals.energy / vitals.max_energy.max(1.0),
            _ => 0.0,
        };
        node.reborrow()
            .map_unchanged(|value| &mut value.width)
            .set_if_neq(percent(fraction.clamp(0.0, 1.0) * 100.0));
    }
    let pressed = hints.iter().find_map(|(interaction, hint)| {
        (*interaction == Interaction::Pressed && interaction.is_changed()).then_some(*hint)
    });
    if session.paused || session.finished {
        review.pinned = None;
    } else if let Some(hint) = pressed {
        review.press(match hint {
            HudHint::BondProgress(id) => Some(id),
            _ => None,
        });
    } else if mouse
        .as_ref()
        .is_some_and(|input| input.just_pressed(MouseButton::Left))
        || touches
            .as_ref()
            .is_some_and(|input| input.iter_just_pressed().next().is_some())
    {
        review.pinned = None;
    }
    let hovered = hints
        .iter()
        .find_map(|(interaction, hint)| (*interaction != Interaction::None).then_some(*hint));
    let hovered_bond = match hovered {
        Some(HudHint::BondProgress(id)) => Some(id),
        _ => None,
    };
    if hovered_bond != review.dismissed {
        review.dismissed = None;
    }
    let hovered = hovered.filter(|hint| {
        !matches!(hint,
        HudHint::BondProgress(id) if review.dismissed == Some(*id))
    });
    let hint = (!session.paused && !session.finished)
        .then_some(hovered.or(review.pinned.map(HudHint::BondProgress)))
        .flatten();
    for mut node in &mut tips {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if hint.is_some() {
                Display::Flex
            } else {
                Display::None
            });
    }
    let Some(hint) = hint else {
        return;
    };
    let copy = match &hint {
        HudHint::Fruit => language.pick("Bond fruit · Recruit a hero\nPreview new heroes. One fruit is used only when you confirm. Absorb the old hero’s innate attributes; keep skills, bonds and equipment.", "羁绊果实 · 英雄征募\n预览新英雄，确认时才消耗一枚果实。吞噬旧英雄固有三维，保留技能、羁绊与装备。").into(),
        HudHint::BondProgress(id) => {
            let mut detail = bond_family_detail(*language, *id, &build);
            if *id == BondId::Archery
                && let Some(runtime) = bonds.as_ref().filter(|runtime| runtime.archery_active()) {
                    detail.push_str(&format!("\n{} {}/{}", language.pick("Quiver", "箭袋"), runtime.arrows_remaining, runtime.quiver_capacity()));
                    if runtime.recharge_remaining > 0.0 {
                        detail.push_str(&format!(" · {} {:.1}{}", language.pick("Reload", "装填"), runtime.recharge_remaining, language.pick("s", "秒")));
                    }
                    detail.push_str(language.pick("\nExtra arrows use the quiver; basic attacks continue during reload.", "\n额外箭矢消耗箭袋；装填期间普通攻击继续。"));
            }
            detail
        },
        HudHint::Bond if super::super::available_bond_cards(&build).is_empty() => language.pick("Collection complete · All available bond cards are owned. Your absorbed family effects remain active.", "已集齐当前全部羁绊卡，已吞噬羁绊的效果继续生效。").to_string(),
        HudHint::Bond if super::super::available_bond_cards_at(&build, &session).is_empty() => language.pick("Next bond pools · Keep progressing\nNew stages open at level 20 / 40 / 50, or after 8 / 15 / 20 minutes. Complete the prerequisite collection to unlock each recipe.", "后继羁绊池 · 继续成长\n新阶段在20／40／50级或8／15／20分钟开放；完成前置组合解锁对应系列。").into(),
        HudHint::Bond => if language.is_chinese() { format!("羁绊 · F\n花费 {} 木材，预览并选择一个羁绊。", bond_cost(&session, &encounters)) }
            else { format!("Bonds · F\nPreview and choose a bond for {} wood.", bond_cost(&session, &encounters)) },
        HudHint::Encounter if encounters.banked == 0 && !encounters.pending() => language.pick("Encounters · E\nDefeat a checkpoint boss (5 / 10 / 15 / 20 min) to bank one. Each offers three events with a benefit and a cost.", "奇遇 · E\n击败 5 / 10 / 15 / 20 分钟的首领各存入一次。每次三选一，每个选项都有收益与代价。").into(),
        HudHint::Encounter => if language.is_chinese() { format!("奇遇 · E\n已存 {} 次。三选一，每个选项都有收益与代价；可稍后再选，不可刷新或放弃。", encounters.banked) }
            else { format!("Encounters · E\n{} banked. Choose one of three events, each with a benefit and a cost. You may decide later; no refresh or abandon.", encounters.banked) },
        HudHint::Skill => if session.skill_points > 0 {
            format!("{}\n{} {}", language.pick("Learn a skill · G", "学习技能 · G"), session.skill_points, language.pick("free choices available. Preview before choosing.", "次免费选择，可先预览。"))
        } else if language.is_chinese() { format!("学习技能 · G\n花费 {} 金币，预览后选择。", skill_cost(&economy)) }
            else { format!("Learn a skill · G\nPreview and choose for {} gold.", skill_cost(&economy)) },
        HudHint::Dodge => language.pick("Dodge · Shift\nDash in your movement direction. Briefly avoid incoming damage.", "闪避 · Shift\n沿移动方向冲刺，短暂避开伤害。").to_string(),
        HudHint::Collection => language.pick("Bond collection · H\nReview every recipe, collected card and unlocked pool.", "羁绊图鉴 · H\n查看全部组合、已收集卡片和解锁条件。").into(),
        HudHint::Lives => language.pick("Commander lives\nIncludes your current life. Each death consumes one. With no lives left, the expedition ends; the gate does not cause defeat.", "英雄生命\n包含当前生命，每次死亡消耗一次。生命耗尽则远征失败；城门倒地不会导致失败。").into(),
        HudHint::Gate => language.pick("Gate health\nBlocks the entire road and takes damage before your commander. When destroyed, a 4–3–2–1 countdown appears at the gate. It rebuilds at full health with no limit on rebuilds. Enemies can cross while it is down.", "城门耐久\n横跨道路阻挡敌人，替后方英雄承受伤害。倒下后原地显示 4、3、2、1 重建倒计时，归零后满耐久重建，次数不限。倒下期间敌人可以通过。").into(),
        HudHint::Attributes => language.pick("Hero attributes\nView current combat attributes and equipment effects.", "英雄属性\n查看当前战斗属性与装备效果。").into(),
        HudHint::Trials => language.pick("Challenges\nOpen tower and gold trials. Each trial offers an optional battle and rewards.", "挑战\n展开爬塔与金币挑战，进行额外战斗以获取奖励。").to_string(),
        HudHint::Pause => language.pick("Pause · Escape\nTake a break or review this run.", "暂停 · Escape\n暂停战斗并查看本局信息。").to_string(),
        HudHint::Trial(action) => match action {
            TrialAction::StartTower => language.pick("Tower trial\nFight a stronger opponent for permanent command growth.", "爬塔挑战\n击败强敌以获取永久指挥成长。"),
            TrialAction::StartGold => language.pick("Gold trial\nDefeat the targets before time runs out for bonus gold.", "金币挑战\n在限时内击败目标，获取额外金币。"),
            TrialAction::ToggleTowerAuto | TrialAction::ToggleGoldAuto => language.pick("Auto challenge\nToggle automatic entry when the trial is ready.", "自动挑战\n开启或关闭挑战就绪时自动进入。"),
            TrialAction::CancelAll => language.pick("Clear challenges\nLeave active optional trials.", "清除挑战\n退出当前的额外挑战。"),
        }.to_string(),
        HudHint::Speed(action) => match action {
            SpeedAction::Decrease => language.pick("Slower · Halve battle speed\nMinimum 1×.", "减速 · 战斗速度减半\n最低一倍速。"),
            SpeedAction::Increase => language.pick("Faster · Double battle speed\nMaximum 32×. T also increases speed.", "加速 · 战斗速度翻倍\n最高三十二倍速，按 T 也可加速。"),
            SpeedAction::Reset => language.pick("Current battle speed\nReset to 1×.", "当前战斗速度\n点击恢复一倍速。"),
        }.into(),
    };
    for mut text in &mut tip_text {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(copy.clone());
    }
}

#[derive(Component)]
pub(super) struct HudBondStrip;

#[derive(Component)]
pub(super) struct HudBondBadge(BondId);

#[derive(Component)]
pub(super) struct HudBondProgress(BondId);

pub(super) fn spawn_bond_strip(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            HudBondStrip,
            Node {
                width: px(206),
                display: Display::None,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(4),
                row_gap: px(3),
                ..default()
            },
        ))
        .with_children(|strip| {
            for id in BondId::ALL {
                strip
                    .spawn((
                        Button,
                        HudBondBadge(id),
                        Name::new(format!("Bond badge {}", id.definition().id)),
                        HudHint::BondProgress(id),
                        Node {
                            display: Display::None,
                            width: px(30),
                            height: px(38),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            flex_shrink: 0.0,
                            border: UiRect::all(px(1)),
                            border_radius: BorderRadius::all(px(4)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.025, 0.035, 0.055, 0.9)),
                        BorderColor::all(Color::srgba(0.4, 0.45, 0.53, 0.6)),
                    ))
                    .with_children(|badge| {
                        hud_icon(badge, assets, id.definition().icon, 24.0);
                        badge.spawn((
                            HudBondProgress(id),
                            Text::new(""),
                            TextFont {
                                font_size: FontSize::Px(9.0),
                                ..default()
                            },
                            TextColor(Color::srgb(0.94, 0.85, 0.63)),
                            Pickable::IGNORE,
                        ));
                    });
            }
        });
}

#[derive(Default)]
pub(super) struct BondBadgePulse {
    seen: [bool; BondId::ALL.len()],
    remaining: [f32; BondId::ALL.len()],
    initialized: bool,
}

#[allow(clippy::type_complexity)]
pub(super) fn update_bond_strip(
    windows: Query<&Window, With<PrimaryWindow>>,
    build: Res<PlayerBuild>,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    mut pulse: Local<BondBadgePulse>,
    mut strips: Query<&mut Node, (With<HudBondStrip>, Without<HudBondBadge>)>,
    mut badges: Query<
        (
            &HudBondBadge,
            &mut Node,
            &mut BorderColor,
            &mut BackgroundColor,
        ),
        Without<HudBondStrip>,
    >,
    mut labels: Query<(&HudBondProgress, &mut Text)>,
) {
    let compact = windows
        .single()
        .is_ok_and(|window| hud_metrics(window.width(), window.height()).compact);
    let visible = BondId::ALL
        .iter()
        .any(|id| build.bond(*id) > 0 || build.bond_complete(*id));
    for mut node in &mut strips {
        node.reborrow()
            .map_unchanged(|value| &mut value.row_gap)
            .set_if_neq(px(if compact { 1 } else { 3 }));
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if visible {
                Display::Flex
            } else {
                Display::None
            });
    }
    // The battle HUD stays one row tall; the collection panel preserves the full list.
    for id in BondId::ALL {
        let index = id.index();
        let complete = build.bond_complete(id);
        if pulse.initialized && complete && !pulse.seen[index] {
            pulse.remaining[index] = 3.0;
        }
        pulse.seen[index] = complete;
        if !session.paused {
            pulse.remaining[index] = (pulse.remaining[index] - time.delta_secs()).max(0.0);
        }
    }
    let mut focused: Vec<_> = BondId::ALL
        .into_iter()
        .filter(|id| {
            pulse.remaining[id.index()] > 0.0 || (build.bond(*id) > 0 && !build.bond_complete(*id))
        })
        .collect();
    focused.sort_by(|a, b| {
        (pulse.remaining[b.index()] > 0.0)
            .cmp(&(pulse.remaining[a.index()] > 0.0))
            .then_with(|| {
                (build.bond(*b) as f32 / b.required() as f32)
                    .total_cmp(&(build.bond(*a) as f32 / a.required() as f32))
            })
            .then_with(|| a.index().cmp(&b.index()))
    });
    focused.truncate(6);
    for (badge, mut node, mut border, mut background) in &mut badges {
        node.reborrow()
            .map_unchanged(|value| &mut value.height)
            .set_if_neq(px(if compact { 36 } else { 38 }));
        let index = badge.0.index();
        let complete = build.bond_complete(badge.0);
        let glow = if pulse.remaining[index] > 0.0 {
            (pulse.remaining[index] * 8.0).sin().abs() * 0.22
        } else {
            0.0
        };
        background.0 = if complete {
            Color::srgba(0.15 + glow, 0.105 + glow * 0.65, 0.035, 0.95)
        } else {
            Color::srgba(0.025, 0.035, 0.055, 0.9)
        };
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if focused.contains(&badge.0) {
                Display::Flex
            } else {
                Display::None
            });
        *border = BorderColor::all(if complete {
            Color::srgb(0.91, 0.69, 0.25)
        } else {
            Color::srgba(0.4, 0.45, 0.53, 0.6)
        });
    }
    pulse.initialized = true;
    for (progress, mut text) in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if build.bond_complete(progress.0) {
                "✓".to_string()
            } else {
                format!("{}/{}", build.bond(progress.0), progress.0.required())
            });
    }
}

#[cfg(test)]
mod bond_review_tests {
    use super::*;

    #[test]
    fn badge_press_pins_switches_and_dismisses_the_review() {
        let mut review = BondReview::default();
        let first = BondId::ALL[0];
        let second = BondId::ALL[1];
        review.press(Some(first));
        assert_eq!(review.pinned, Some(first));
        review.press(Some(second));
        assert_eq!(review.pinned, Some(second));
        review.press(Some(second));
        assert_eq!(review.pinned, None);
        assert_eq!(review.dismissed, Some(second));
        review.press(Some(first));
        review.press(None);
        assert_eq!(review.pinned, None);
    }
}

// Gold focus states match the recruitment cards without changing button actions.
pub(super) fn update_small_controls(
    drawer: Res<TrialDrawerState>,
    mut buttons: Query<(
        &HudHint,
        &Interaction,
        &mut BackgroundColor,
        &mut BorderColor,
    )>,
) {
    for (hint, interaction, mut background, mut border) in &mut buttons {
        if !matches!(hint, HudHint::Pause | HudHint::Trials) {
            continue;
        }
        let focused =
            *interaction != Interaction::None || (matches!(hint, HudHint::Trials) && drawer.open);
        background.0 = if focused {
            Color::srgba(0.26, 0.18, 0.075, 0.96)
        } else {
            Color::srgba(0.035, 0.045, 0.065, 0.95)
        };
        *border = BorderColor::all(if focused {
            Color::srgb(0.92, 0.72, 0.34)
        } else {
            Color::srgba(0.65, 0.49, 0.25, 0.75)
        });
    }
}

#[derive(Component)]
pub(super) struct FruitRecruitButton;
#[derive(Component)]
pub(super) struct FruitCount;

pub(super) fn spawn_fruit_button(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Button,
            FruitRecruitButton,
            HudHint::Fruit,
            Name::new("Fruit recruit"),
            Node {
                display: Display::None,
                width: px(104),
                flex_shrink: 0.0,
                height: px(44),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                column_gap: px(8),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(7)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.06, 0.075, 0.07)),
            BorderColor::all(Color::srgb(0.65, 0.49, 0.25)),
        ))
        .with_children(|button| {
            hud_icon(button, assets, "transmute_spark", 28.0);
            button.spawn((
                FruitCount,
                Text::new("0"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::srgb(0.95, 0.85, 0.61)),
            ));
        });
}

#[derive(Component)]
pub(super) struct EncounterCount;

/// Sits beside the F button; shows the banked count and dims when nothing is banked.
pub(super) fn spawn_encounter_button(parent: &mut ChildSpawnerCommands, assets: &AssetServer) {
    parent
        .spawn((
            Button,
            EncounterButton,
            HudHint::Encounter,
            Name::new("Encounter control"),
            Node {
                width: px(104),
                height: px(44),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(7)),
                column_gap: px(8),
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.055, 0.065, 0.085, 0.94)),
            BorderColor::all(Color::srgba(0.65, 0.49, 0.25, 0.8)),
        ))
        .with_children(|button| {
            hud_icon(button, assets, "fate_cube", 30.0);
            button.spawn((
                EncounterButtonLabel,
                EncounterCount,
                Text::new("0"),
                TextFont {
                    font_size: FontSize::Px(13.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

pub(super) fn update_encounter_button(
    encounters: Res<EncounterState>,
    mut buttons: Query<(&mut BackgroundColor, &mut BorderColor), With<EncounterButton>>,
    mut labels: Query<(&mut Text, &mut TextColor), With<EncounterCount>>,
) {
    let ready = encounters.banked > 0 || encounters.pending();
    for (mut background, mut border) in &mut buttons {
        background.0 = if ready {
            Color::srgba(0.16, 0.11, 0.2, 0.96)
        } else {
            Color::srgba(0.055, 0.065, 0.085, 0.5)
        };
        *border = BorderColor::all(if ready {
            Color::srgb(0.78, 0.6, 0.95)
        } else {
            Color::srgba(0.65, 0.49, 0.25, 0.3)
        });
    }
    for (mut text, mut color) in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(encounters.banked.to_string());
        color
            .reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(if ready {
                Color::WHITE
            } else {
                Color::srgba(1.0, 1.0, 1.0, 0.4)
            });
    }
}

pub(super) fn update_fruit_button(
    mut recruitment: ResMut<super::super::RecruitmentProgress>,
    session: Res<GameSession>,
    mut buttons: Query<(&Interaction, &mut Node), With<FruitRecruitButton>>,
    mut labels: Query<&mut Text, With<FruitCount>>,
) {
    for (interaction, mut node) in &mut buttons {
        node.reborrow()
            .map_unchanged(|value| &mut value.display)
            .set_if_neq(if recruitment.fruit_count > 0 {
                Display::Flex
            } else {
                Display::None
            });
        if *interaction == Interaction::Pressed && !session.paused && recruitment.pending.is_none()
        {
            recruitment.requested = true;
        }
    }
    for mut text in &mut labels {
        text.reborrow()
            .map_unchanged(|value| &mut value.0)
            .set_if_neq(recruitment.fruit_count.to_string());
    }
}
