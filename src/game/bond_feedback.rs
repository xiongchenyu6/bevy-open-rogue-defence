//! Celebrates newly completed collections without repeating gameplay rewards.
use super::attack_vfx::WeaponCues;
use super::audio_mix::{AudioChannel, AudioVoice};
use super::bond_catalog::BondEffect;
use super::localization::{UI_FONT_PATH, bond_name};
use super::{Autoplay, BondId, GameLanguage, GameSession, Hero, HeroKind, PlayerBuild};
use bevy::{audio::Volume, prelude::*};
use std::collections::VecDeque;

const DURATION: f32 = 4.8;

pub(crate) struct BondFeedbackPlugin;
impl Plugin for BondFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BondCelebrationState>().add_systems(
            PostUpdate,
            (observe_completions, tick_celebration, show_next)
                .chain()
                .in_set(BondFeedbackSet)
                .before(TransformSystems::Propagate),
        );
    }
}
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct BondFeedbackSet;
#[derive(Resource, Default)]
pub(crate) struct BondCelebrationState {
    initialized: bool,
    seen: [bool; BondId::COUNT],
    pending: VecDeque<BondId>,
    active: bool,
}
impl BondCelebrationState {
    pub(crate) fn busy(&self) -> bool {
        self.active || !self.pending.is_empty()
    }
}
#[derive(Component)]
struct Celebration {
    family: BondId,
    age: f32,
}
#[derive(Component)]
struct FusionSeal {
    glow: bool,
}
#[derive(Component)]
struct MemberSeal {
    index: usize,
    count: usize,
}
#[derive(Component)]
enum CelebrationCopy {
    Title,
    Effect,
    Reward,
}

fn observe_completions(
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    mut state: ResMut<BondCelebrationState>,
) {
    if !state.initialized || session.finished {
        state.initialized = true;
        state.seen = BondId::ALL.map(|id| build.bond_complete(id));
        state.pending.clear();
        return;
    }
    for family in BondId::ALL {
        if build.bond_complete(family) && !state.seen[family.index()] {
            state.seen[family.index()] = true;
            state.pending.push_back(family);
        }
    }
}

fn fruit_reward(family: BondId) -> u32 {
    family
        .definition()
        .bonuses
        .iter()
        .filter(|bonus| bonus.required == family.required())
        .flat_map(|bonus| bonus.effects)
        .filter_map(|effect| {
            if let BondEffect::JobFruit(count) = effect {
                Some(*count)
            } else {
                None
            }
        })
        .sum()
}
fn copy(language: GameLanguage, family: BondId, kind: &CelebrationCopy) -> String {
    match kind {
        CelebrationCopy::Title => format!(
            "{}  ·  {}",
            bond_name(language, family),
            language.pick("BOND COMPLETE", "羁绊集齐")
        ),
        CelebrationCopy::Effect => family
            .definition()
            .bonuses
            .iter()
            .filter(|bonus| bonus.required == family.required())
            .map(|bonus| language.pick(bonus.description_en, bonus.description))
            .collect::<Vec<_>>()
            .join(" "),
        CelebrationCopy::Reward => {
            let fruit = fruit_reward(family);
            if fruit > 0 {
                format!(
                    "+{fruit} {}  ·  {}",
                    language.pick("JOB FRUIT", "转职果实"),
                    language.pick("READY TO RECRUIT", "可用于转职")
                )
            } else {
                let children = BondId::ALL
                    .into_iter()
                    .filter(|id| id.definition().prerequisite == Some(family))
                    .map(|id| bond_name(language, id))
                    .collect::<Vec<_>>();
                if children.is_empty() {
                    language
                        .pick(
                            "POWER RETAINED · FAMILY SLOT FREED",
                            "保留能力 · 释放羁绊槽",
                        )
                        .to_string()
                } else {
                    format!(
                        "{}  {}",
                        language.pick("POOL UNLOCKED", "解锁羁绊池"),
                        children.join(" / ")
                    )
                }
            }
        }
    }
}
fn layout(windows: &Query<&Window>) -> (bool, f32, f32) {
    let compact = windows.iter().next().is_some_and(|window| {
        super::mobile_layout::hud_metrics(window.width(), window.height()).compact
    });
    (
        compact,
        if compact { 280.0 } else { 470.0 },
        if compact { 116.0 } else { 70.0 },
    )
}
fn tick_celebration(
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    mut state: ResMut<BondCelebrationState>,
    mut panels: Query<
        (
            Entity,
            &mut Celebration,
            &mut Node,
            &mut Visibility,
            &mut BorderColor,
        ),
        (Without<MemberSeal>, Without<FusionSeal>),
    >,
    mut members: Query<
        (&MemberSeal, &mut Node, &mut ImageNode),
        (Without<Celebration>, Without<FusionSeal>),
    >,
    mut seals: Query<
        (
            &FusionSeal,
            &mut Node,
            Option<&mut ImageNode>,
            Option<&mut BorderColor>,
        ),
        (Without<Celebration>, Without<MemberSeal>),
    >,
    mut labels: Query<(&CelebrationCopy, &mut Text, &mut Visibility), Without<Celebration>>,
    windows: Query<&Window>,
) {
    let (compact, width, top) = layout(&windows);
    for (entity, mut celebration, mut node, mut visibility, mut border) in &mut panels {
        if session.finished {
            commands.entity(entity).despawn();
            state.active = false;
            continue;
        }
        *visibility = if session.paused {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if session.paused {
            continue;
        }
        celebration.age += time.delta_secs();
        if celebration.age >= DURATION {
            commands.entity(entity).despawn();
            state.active = false;
            continue;
        }
        let arrive = (celebration.age / 0.6).clamp(0.0, 1.0);
        let leave = ((celebration.age - (DURATION - 0.35)) / 0.35).clamp(0.0, 1.0);
        node.top = px(top - 12.0 * (1.0 - arrive).powi(2) - leave * 12.0);
        node.width = px(width);
        node.left = percent(if compact { 60.0 } else { 50.0 });
        node.margin.left = px(-width * 0.5);
        let gleam = (1.0 - arrive) * 0.3;
        *border = BorderColor::all(Color::srgb(1.0, 0.72 + gleam, 0.35 + gleam));
        let icon = if compact { 24.0 } else { 34.0 };
        let seal_size = if compact { 40.0 } else { 64.0 };
        let fusion = ((celebration.age - 0.15) / 0.5).clamp(0.0, 1.0);
        let reveal = ((celebration.age - 0.5) / 0.3).clamp(0.0, 1.0);
        let center = (width - if compact { 12.0 } else { 24.0 }) * 0.5;
        for (seal, mut seal_node, mut image) in &mut members {
            let offset = seal.index as f32 - (seal.count as f32 - 1.0) * 0.5;
            seal_node.left =
                px(center + offset * (icon + 16.0) * (1.0 - fusion).powi(2) - icon * 0.5);
            seal_node.top = px((seal_size - icon) * 0.5);
            image.color = Color::srgba(1.0, 1.0, 1.0, 1.0 - fusion);
        }
        for (seal, mut seal_node, image, border) in &mut seals {
            let overshoot = (reveal * std::f32::consts::PI).sin() * 0.22;
            let size =
                seal_size * (0.6 + 0.4 * reveal + overshoot) * if seal.glow { 1.08 } else { 1.0 };
            seal_node.width = px(size);
            seal_node.height = px(size);
            seal_node.left = px(center - size * 0.5);
            seal_node.top = px((seal_size - size) * 0.5);
            if let Some(mut image) = image {
                image.color = Color::srgba(1.0, 1.0, 1.0, reveal);
            }
            if let Some(mut border) = border {
                *border =
                    BorderColor::all(Color::srgba(1.0, 0.76, 0.32, reveal * (0.4 + overshoot)));
            }
        }
        for (kind, mut text, mut visibility) in &mut labels {
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(copy(*language, celebration.family, kind));
            *visibility = if matches!(kind, CelebrationCopy::Reward) && celebration.age < 0.7 {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
        }
    }
}
fn show_next(
    mut commands: Commands,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    mut state: ResMut<BondCelebrationState>,
    server: Option<Res<AssetServer>>,
    hero: Query<&Transform, With<Hero>>,
    mut cues: Option<ResMut<WeaponCues>>,
    autoplay: Option<Res<Autoplay>>,
    windows: Query<&Window>,
) {
    if session.paused || session.finished || state.active {
        return;
    }
    let Some(server) = server else {
        return;
    };
    let Some(family) = state.pending.pop_front() else {
        return;
    };
    state.active = true;
    let (compact, width, top) = layout(&windows);
    let icon = if compact { 24.0 } else { 34.0 };
    let font: Handle<Font> = server.load(UI_FONT_PATH);
    commands
        .spawn((
            Name::new("Bond completion celebration"),
            Celebration { family, age: 0.0 },
            Node {
                position_type: PositionType::Absolute,
                top: px(top),
                left: percent(if compact { 60.0 } else { 50.0 }),
                margin: UiRect::left(px(-width * 0.5)),
                width: px(width),
                padding: UiRect::all(px(if compact { 6.0 } else { 12.0 })),
                row_gap: px(if compact { 3.0 } else { 6.0 }),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::vertical(px(2)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.045, 0.048, 0.93)),
            BorderColor::all(Color::srgb(1.0, 0.76, 0.4)),
            GlobalZIndex(95),
            Pickable::IGNORE,
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(if compact { 40.0 } else { 64.0 }),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|row| {
                    row.spawn((
                        FusionSeal { glow: true },
                        Node {
                            position_type: PositionType::Absolute,
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(percent(50)),
                            ..default()
                        },
                        BorderColor::all(Color::NONE),
                        Pickable::IGNORE,
                    ));
                    row.spawn((
                        FusionSeal { glow: false },
                        ImageNode {
                            color: Color::NONE,
                            ..ImageNode::new(
                                server.load(format!("ui/icons/{}.png", family.definition().icon)),
                            )
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                        Pickable::IGNORE,
                    ));
                    for (index, card) in family.definition().members.iter().enumerate() {
                        row.spawn((
                            MemberSeal {
                                index,
                                count: family.definition().members.len(),
                            },
                            ImageNode::new(
                                server.load(format!("ui/icons/{}.png", card.definition().icon)),
                            ),
                            Node {
                                position_type: PositionType::Absolute,
                                width: px(icon),
                                height: px(icon),
                                ..default()
                            },
                            Pickable::IGNORE,
                        ));
                    }
                });
            for kind in [
                CelebrationCopy::Title,
                CelebrationCopy::Effect,
                CelebrationCopy::Reward,
            ] {
                let (name, size, color) = match kind {
                    CelebrationCopy::Title => (
                        "Bond completion title",
                        if compact { 14.0 } else { 20.0 },
                        Color::srgb(1.0, 0.84, 0.46),
                    ),
                    CelebrationCopy::Effect => (
                        "Bond completion effect",
                        if compact { 10.5 } else { 13.0 },
                        Color::srgb(0.87, 0.91, 0.88),
                    ),
                    CelebrationCopy::Reward => (
                        "Bond completion reward",
                        if compact { 10.5 } else { 13.0 },
                        Color::srgb(1.0, 0.78, 0.35),
                    ),
                };
                panel.spawn((
                    Name::new(name),
                    Visibility::Inherited,
                    Text::new(copy(*language, family, &kind)),
                    kind,
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::Px(size),
                        ..default()
                    },
                    Node {
                        width: percent(100),
                        ..default()
                    },
                    TextLayout::justify(Justify::Center),
                    TextColor(color),
                    Pickable::IGNORE,
                ));
            }
        });
    if let (Some(cues), Ok(hero)) = (cues.as_deref_mut(), hero.single()) {
        for direction in [Vec3::X, Vec3::Z, Vec3::NEG_X, Vec3::NEG_Z] {
            cues.push(
                HeroKind::SolarVanguard,
                hero.translation + direction * 0.7,
                direction,
                true,
            );
        }
    }
    if autoplay.is_none() {
        commands.spawn((
            AudioPlayer::new(server.load("audio/bond_complete.wav")),
            AudioVoice::new(AudioChannel::Effects, 0.55),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.55)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn finish(build: &mut PlayerBuild, family: BondId) {
        for card in family.definition().members {
            build.owned_bond_cards[card.index()] = true;
        }
    }
    #[test]
    fn completion_queues_once_ignores_baseline_and_does_not_regrant_rewards() {
        let mut app = App::new();
        app.init_resource::<PlayerBuild>()
            .init_resource::<GameSession>()
            .init_resource::<BondCelebrationState>()
            .add_systems(Update, observe_completions);
        finish(
            &mut app.world_mut().resource_mut::<PlayerBuild>(),
            BondId::Journey,
        );
        app.update();
        assert!(!app.world().resource::<BondCelebrationState>().busy());
        app.world_mut().resource_mut::<GameSession>().paused = true;
        finish(
            &mut app.world_mut().resource_mut::<PlayerBuild>(),
            BondId::Warpath,
        );
        finish(
            &mut app.world_mut().resource_mut::<PlayerBuild>(),
            BondId::Arcanum,
        );
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<BondCelebrationState>().pending.len(),
            2
        );
        let card = BondId::Warpath.definition().members[0];
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards[card.index()] = false;
        app.update();
        finish(
            &mut app.world_mut().resource_mut::<PlayerBuild>(),
            BondId::Warpath,
        );
        app.update();
        assert_eq!(
            app.world().resource::<BondCelebrationState>().pending.len(),
            2
        );
        app.world_mut().resource_mut::<GameSession>().finished = true;
        app.update();
        assert!(
            app.world()
                .resource::<BondCelebrationState>()
                .pending
                .is_empty()
        );
    }
    #[test]
    fn celebration_keeps_real_time_duration_at_triple_speed_and_waits_through_pause() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<GameLanguage>()
            .init_resource::<BondCelebrationState>()
            .init_resource::<Time<Real>>()
            .init_resource::<Time<Virtual>>()
            .add_systems(Update, tick_celebration);
        app.world_mut()
            .resource_mut::<Time<Virtual>>()
            .set_relative_speed(3.0);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_secs(1));
        app.world_mut()
            .resource_mut::<BondCelebrationState>()
            .active = true;
        let notice = app
            .world_mut()
            .spawn((
                Celebration {
                    family: BondId::Journey,
                    age: 0.0,
                },
                Node::default(),
                Visibility::Inherited,
                BorderColor::default(),
            ))
            .id();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(app.world().get::<Celebration>(notice).unwrap().age, 4.0);
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.update();
        assert_eq!(app.world().get::<Celebration>(notice).unwrap().age, 4.0);
        assert_eq!(
            *app.world().get::<Visibility>(notice).unwrap(),
            Visibility::Hidden
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        assert!(app.world().get_entity(notice).is_err());
        assert!(!app.world().resource::<BondCelebrationState>().active);
    }
    #[test]
    fn completion_copy_uses_real_bonus_unlocks_and_fruit_rewards() {
        let en = GameLanguage::English;
        let zh = GameLanguage::SimplifiedChinese;
        let unlocks = copy(en, BondId::Journey, &CelebrationCopy::Reward);
        for family in [BondId::Strength, BondId::Agility, BondId::Intelligence] {
            assert!(unlocks.contains(&bond_name(en, family)));
        }
        assert_eq!(fruit_reward(BondId::Journey), 0);
        assert_eq!(fruit_reward(BondId::Warpath), 1);
        assert!(copy(zh, BondId::Warpath, &CelebrationCopy::Reward).contains("+1 转职果实"));
        assert_eq!(
            copy(en, BondId::Warpath, &CelebrationCopy::Effect),
            BondId::Warpath.definition().bonuses[0].description_en
        );
    }
}
