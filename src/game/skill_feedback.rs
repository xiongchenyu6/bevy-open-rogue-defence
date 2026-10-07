//! Persistent rank readability and brief, non-interactive upgrade confirmation.
use super::attack_vfx::WeaponCues;
use super::audio_mix::{AudioChannel, AudioVoice};
use super::bonds::DamageProfile;
use super::content::SKILLS;
use super::localization::{UI_FONT_PATH, skill_name};
use super::{
    Autoplay, GameLanguage, GameSession, Hero, HeroKind, PlayerBuild, Projectile, SkillId,
};
use bevy::{audio::Volume, prelude::*};
use std::collections::VecDeque;

pub(crate) struct SkillFeedbackPlugin;
impl Plugin for SkillFeedbackPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RankFeedback>().add_systems(
            PostUpdate,
            (
                observe_ranks,
                tick_notice,
                show_next_notice,
                size_skill_projectiles,
            )
                .chain()
                .after(super::bond_feedback::BondFeedbackSet)
                .before(TransformSystems::Propagate),
        );
    }
}

#[derive(Clone, Copy)]
struct Upgrade {
    skill: usize,
    from: u8,
    to: u8,
    ascension: bool,
    path: Option<super::skill_evolution::UrEvolutionPath>,
}
#[derive(Resource, Default)]
struct RankFeedback {
    initialized: bool,
    ranks: [u8; SkillId::COUNT],
    tiers: [u8; SkillId::COUNT],
    paths: [Option<super::skill_evolution::UrEvolutionPath>; SkillId::COUNT],
    pending: VecDeque<Upgrade>,
}
#[derive(Component)]
struct SkillNotice {
    upgrade: Upgrade,
    remaining: f32,
}
#[derive(Component)]
struct NoticeText;

fn observe_ranks(
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    mut state: ResMut<RankFeedback>,
    evolution: Option<Res<super::skill_evolution::SkillEvolution>>,
) {
    let tiers = evolution.as_ref().map_or([0; SkillId::COUNT], |e| e.tiers);
    let paths = evolution
        .as_ref()
        .map_or([None; SkillId::COUNT], |e| e.paths);
    if !state.initialized || session.finished {
        state.initialized = true;
        state.ranks = build.skills;
        state.tiers = tiers;
        state.paths = paths;
        state.pending.clear();
        return;
    }
    for (index, &rank) in build.skills.iter().enumerate() {
        let before = state.ranks[index];
        if rank > before {
            if let Some(pending) = state
                .pending
                .iter_mut()
                .find(|change| change.skill == index && !change.ascension)
            {
                pending.to = rank;
            } else {
                state.pending.push_back(Upgrade {
                    skill: index,
                    from: before,
                    to: rank,
                    ascension: false,
                    path: None,
                });
            }
        } else if rank < before {
            state.pending.retain(|change| change.skill != index);
        }
    }
    for (skill, &tier) in tiers.iter().enumerate() {
        if tier > state.tiers[skill] && build.skills[skill] > 0 {
            let from = state.tiers[skill];
            state.pending.push_back(Upgrade {
                skill,
                from,
                to: tier,
                ascension: true,
                path: None,
            });
        }
    }
    for (skill, &path) in paths.iter().enumerate() {
        if path.is_some() && path != state.paths[skill] && build.skills[skill] > 0 {
            state.pending.push_back(Upgrade {
                skill,
                from: 0,
                to: 1,
                ascension: false,
                path,
            });
        }
    }
    state.ranks = build.skills;
    state.tiers = tiers;
    state.paths = paths;
}

fn notice_copy(language: GameLanguage, change: Upgrade) -> String {
    let name = skill_name(language, SkillId(change.skill as u8));
    if let Some(path) = change.path {
        return format!(
            "{} · {}\n{}",
            name,
            language.pick("EVOLVED", "进化完成"),
            super::ur_evolution::name(language, SkillId(change.skill as u8), path)
        );
    }
    let level = if change.ascension {
        language.pick("ASCENSION", "升华")
    } else {
        language.pick("LEVEL", "等级")
    };
    if change.from == 0 {
        format!("{name}\n+ {level} {}", change.to)
    } else {
        format!("{name}\n{level} {} → {}", change.from, change.to)
    }
}

fn notice_top(width: f32, height: f32) -> f32 {
    if super::mobile_layout::hud_metrics(width, height).compact {
        (height - 175.0).max(120.0)
    } else {
        66.0
    }
}

fn tick_notice(
    celebration: Option<Res<super::bond_feedback::BondCelebrationState>>,
    mut commands: Commands,
    time: Res<Time<Real>>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    mut notices: Query<(Entity, &mut SkillNotice, &mut Node, &mut Visibility)>,
    mut texts: Query<&mut Text, With<NoticeText>>,
    windows: Query<&Window>,
) {
    for (entity, mut notice, mut node, mut visibility) in &mut notices {
        if session.finished {
            commands.entity(entity).despawn();
            continue;
        }
        let defer = session.paused || celebration.as_ref().is_some_and(|state| state.busy());
        *visibility = if defer {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if defer {
            continue;
        }
        notice.remaining -= time.delta_secs();
        if notice.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        let intro = ((2.2 - notice.remaining) / 0.22).clamp(0.0, 1.0);
        let top = windows
            .iter()
            .next()
            .map_or(66.0, |window| notice_top(window.width(), window.height()));
        node.top = px(top + 10.0 * (1.0 - intro).powi(2));
        for mut text in &mut texts {
            text.reborrow()
                .map_unchanged(|value| &mut value.0)
                .set_if_neq(notice_copy(*language, notice.upgrade));
        }
    }
}

fn show_next_notice(
    output: Option<Res<bevy::audio::AudioOutputStatus>>,
    celebration: Option<Res<super::bond_feedback::BondCelebrationState>>,
    mut commands: Commands,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    mut state: ResMut<RankFeedback>,
    server: Option<Res<AssetServer>>,
    notices: Query<(), With<SkillNotice>>,
    hero: Query<&Transform, With<Hero>>,
    mut cues: Option<ResMut<WeaponCues>>,
    autoplay: Option<Res<Autoplay>>,
    windows: Query<&Window>,
) {
    if session.paused
        || session.finished
        || !notices.is_empty()
        || celebration.as_ref().is_some_and(|state| state.busy())
    {
        return;
    }
    let Some(server) = server else {
        return;
    };
    let Some(change) = state.pending.pop_front() else {
        return;
    };
    let compact = windows.iter().next().is_some_and(|window| {
        super::mobile_layout::hud_metrics(window.width(), window.height()).compact
    });
    let width = if compact { 200.0 } else { 220.0 };
    let icon = if compact { 32.0 } else { 40.0 };
    let font: Handle<Font> = server.load(UI_FONT_PATH);
    commands
        .spawn((
            Name::new("Skill upgrade notice"),
            SkillNotice {
                upgrade: change,
                remaining: 2.2,
            },
            Node {
                position_type: PositionType::Absolute,
                top: px(windows
                    .iter()
                    .next()
                    .map_or(66.0, |window| notice_top(window.width(), window.height()))
                    + 10.0),
                left: percent(50),
                margin: UiRect::left(px(-width * 0.5)),
                width: px(width),
                min_height: px(if compact { 48.0 } else { 58.0 }),
                padding: UiRect::all(px(8)),
                column_gap: px(10),
                align_items: AlignItems::Center,
                border: UiRect::all(px(1)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.055, 0.065, 0.94)),
            BorderColor::all(Color::srgb(0.95, 0.75, 0.36)),
            GlobalZIndex(90),
            Pickable::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((
                ImageNode::new(server.load(format!("ui/icons/{}.png", SKILLS[change.skill].id))),
                Node {
                    width: px(icon),
                    height: px(icon),
                    flex_shrink: 0.0,
                    ..default()
                },
                Pickable::IGNORE,
            ));
            parent.spawn((
                NoticeText,
                Text::new(notice_copy(*language, change)),
                TextFont {
                    font: font.into(),
                    font_size: FontSize::Px(if compact { 12.0 } else { 14.0 }),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.89, 0.62)),
                Pickable::IGNORE,
            ));
        });
    if let (Some(cues), Ok(hero)) = (cues.as_deref_mut(), hero.single()) {
        cues.push(HeroKind::SolarVanguard, hero.translation, Vec3::X, true);
    }
    if autoplay.is_none() && output.as_ref().is_none_or(|status| status.is_available()) {
        commands.spawn((
            AudioPlayer::new(server.load("audio/draft.wav")),
            AudioVoice::new(AudioChannel::Effects, 0.45),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(0.45)),
        ));
    }
}

fn projectile_rank_scale(rank: u8) -> f32 {
    1.0 + f32::from(rank.clamp(1, 5) - 1) * 0.13
}
fn size_skill_projectiles(
    build: Res<PlayerBuild>,
    mut projectiles: Query<(&DamageProfile, &mut Transform), (Added<Projectile>, With<Mesh3d>)>,
) {
    for (profile, mut transform) in &mut projectiles {
        if let Some(index) = profile
            .skill_index
            .and_then(|index| build.skills.get(index as usize))
        {
            // Presentation only; never enlarge collision radius or change damage.
            transform.scale *= projectile_rank_scale(*index);
        }
    }
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(RankFeedback::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrades_queue_once_merge_while_paused_and_ignore_removed_skills() {
        let mut app = App::new();
        app.init_resource::<PlayerBuild>()
            .init_resource::<GameSession>()
            .init_resource::<RankFeedback>()
            .add_systems(Update, observe_ranks);
        app.update();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 1;
        app.update();
        app.update();
        assert_eq!(app.world().resource::<RankFeedback>().pending.len(), 1);
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 3;
        app.update();
        let change = app.world().resource::<RankFeedback>().pending[0];
        assert_eq!((change.skill, change.from, change.to), (4, 0, 3));
        assert!(notice_copy(GameLanguage::SimplifiedChinese, change).contains("等级 3"));
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 0;
        app.update();
        assert!(app.world().resource::<RankFeedback>().pending.is_empty());
    }
    #[test]
    fn new_catalog_skills_and_mastery_emit_distinct_notices() {
        let mut app = App::new();
        app.init_resource::<PlayerBuild>()
            .init_resource::<GameSession>()
            .init_resource::<RankFeedback>()
            .init_resource::<super::super::skill_evolution::SkillEvolution>()
            .add_systems(Update, observe_ranks);
        app.update();
        app.world_mut().resource_mut::<PlayerBuild>().skills[SkillId::COUNT - 1] = 5;
        app.update();
        app.world_mut()
            .resource_mut::<super::super::skill_evolution::SkillEvolution>()
            .tiers[SkillId::COUNT - 1] = 1;
        app.update();
        let state = app.world().resource::<RankFeedback>();
        assert_eq!(state.pending.len(), 2);
        assert!(state.pending[1].ascension);
        assert!(notice_copy(GameLanguage::SimplifiedChinese, state.pending[1]).contains("升华"));
    }
    #[test]
    fn projectile_rank_size_is_applied_once_and_pool_reuse_starts_fresh() {
        let mut app = App::new();
        app.init_resource::<PlayerBuild>()
            .add_systems(Update, size_skill_projectiles);
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 5;
        let projectile = || Projectile {
            velocity: Vec3::X,
            damage: 17.0,
            radius: 0.4,
            remaining_hits: 0,
            lifetime: 1.0,
        };
        let entity = app
            .world_mut()
            .spawn((
                projectile(),
                Mesh3d::default(),
                DamageProfile::for_skill(4),
                Transform::from_scale(Vec3::splat(0.72)),
            ))
            .id();
        app.update();
        let size = app.world().get::<Transform>(entity).unwrap().scale;
        assert!((size.x - 0.72 * 1.52).abs() < 0.0001);
        app.update();
        assert_eq!(app.world().get::<Transform>(entity).unwrap().scale, size);
        assert_eq!(app.world().get::<Projectile>(entity).unwrap().radius, 0.4);
        app.world_mut().entity_mut(entity).remove::<Projectile>();
        app.update();
        app.world_mut().resource_mut::<PlayerBuild>().skills[4] = 1;
        app.world_mut()
            .entity_mut(entity)
            .insert((projectile(), Transform::from_scale(Vec3::splat(0.72))));
        app.update();
        assert_eq!(
            app.world().get::<Transform>(entity).unwrap().scale,
            Vec3::splat(0.72)
        );
    }
}
