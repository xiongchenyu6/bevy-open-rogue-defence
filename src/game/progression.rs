use bevy::prelude::*;

use super::content::{BONDS, HEROES, Rarity, SKILLS, SkillEffectKind, Tag};
use super::market::MarketEffects;
use super::{
    Autoplay, BondDrawButton, BondId, DraftAction, DraftActionButton, DraftButton, DraftChoice,
    DraftKind, DraftRoot, DraftState, GameSession, PlayerBuild, RunRng, SelectedHero,
    SkillDrawButton, SkillId,
};

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SkillEconomy>()
            .init_resource::<BondEconomy>()
            .init_resource::<DraftAntiDrought>()
            .add_systems(Startup, initialize_skill_economy)
            .add_systems(
                Update,
                (
                    check_level_up,
                    grant_free_skill_draws,
                    grant_first_free_bond_draw,
                    request_skill_draft,
                    request_bond_draft,
                    handle_draft_buttons,
                    handle_draft_actions,
                    handle_draft_keys,
                    autoplay_draft,
                )
                    .chain(),
            );
    }
}

// The current full-run footage presents its guided G draft at ~00:04.
const FIRST_FREE_SKILL_AT: f32 = 4.0;
// The current full-run footage opens its first guided F draft at ~00:12.
const FIRST_FREE_BOND_AT: f32 = 12.0;
const EPIC_SKILL_UNLOCK_AT: f32 = 12.0 * 60.0;
const FREE_SKILL_LEVELS: [u32; 5] = [5, 10, 15, 20, 25];
const SKILL_SLOT_LIMIT: usize = 6;
const BOND_SLOT_LIMIT: usize = 7;
const REPLACEMENT_GRID_WIDTH_PERCENT: f32 = 94.0;
const REPLACEMENT_GRID_MAX_WIDTH: f32 = 1080.0;
const REPLACEMENT_GRID_HEIGHT: f32 = 320.0;
const REPLACEMENT_CARD_WIDTH_PERCENT: f32 = 30.0;
const REPLACEMENT_CARD_MIN_WIDTH: f32 = 170.0;
const REPLACEMENT_CARD_MIN_HEIGHT: f32 = 96.0;
const REPLACEMENT_COLUMN_GAP: f32 = 12.0;
const REPLACEMENT_ROW_GAP: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkillDrawPayment {
    Free,
    Gold(u32),
}

#[derive(Resource, Debug, Default)]
pub struct SkillEconomy {
    pub paid_draws: u32,
    pub first_free_granted: bool,
    pub level_grants: u8,
    pending_payment: Option<SkillDrawPayment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BondDrawPayment {
    Free,
    Wood(u32),
}

/// Owns the banked 2:00 grant and the in-flight F payment. `GameSession::bond_draws`
/// remains the paid-draw counter because the existing HUD reads it for the next price.
#[derive(Resource, Debug, Default)]
struct BondEconomy {
    first_free_granted: bool,
    free_draw_available: bool,
    pending_payment: Option<BondDrawPayment>,
}

#[derive(Resource, Debug, Default)]
struct DraftAntiDrought {
    skill_missed_preference: bool,
    bond_missed_preference: bool,
}

pub fn skill_cost(economy: &SkillEconomy) -> u32 {
    80_u32
        .saturating_add(economy.paid_draws.saturating_mul(15))
        .min(260)
}

fn initialize_skill_economy(mut session: ResMut<GameSession>) {
    // GameSession keeps the legacy field for HUD compatibility. The economy owns
    // all grants, so the old default point must not be spendable before 0:04.
    session.skill_points = 0;
}

fn check_level_up(mut session: ResMut<GameSession>) {
    if session.finished
        || session.paused
        || session.save_active
        || session.level >= 30
        || session.xp < session.xp_next
    {
        return;
    }
    session.xp -= session.xp_next;
    session.level += 1;
    session.xp_next = 60 + 18 * (session.level - 1);
}

fn grant_free_skill_draws(mut session: ResMut<GameSession>, mut economy: ResMut<SkillEconomy>) {
    if !economy.first_free_granted && session.elapsed >= FIRST_FREE_SKILL_AT {
        economy.first_free_granted = true;
        session.skill_points = session.skill_points.saturating_add(1);
    }

    for (index, level) in FREE_SKILL_LEVELS.into_iter().enumerate() {
        let bit = 1 << index;
        if session.level >= level && economy.level_grants & bit == 0 {
            economy.level_grants |= bit;
            session.skill_points = session.skill_points.saturating_add(1);
        }
    }
}

fn grant_first_free_bond_draw(session: Res<GameSession>, mut economy: ResMut<BondEconomy>) {
    if !economy.first_free_granted && session.elapsed >= FIRST_FREE_BOND_AT {
        economy.first_free_granted = true;
        economy.free_draw_available = true;
    }
}

fn request_skill_draft(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    autoplay: Option<Res<Autoplay>>,
    interactions: Query<&Interaction, (With<SkillDrawButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut economy: ResMut<SkillEconomy>,
    mut rng: ResMut<RunRng>,
    selected: Res<SelectedHero>,
    mut drought: ResMut<DraftAntiDrought>,
) {
    if session.finished || session.paused || session.save_active || draft.root.is_some() {
        return;
    }
    let pressed = keyboard.just_pressed(KeyCode::KeyG)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
        || (autoplay.is_some()
            && session.elapsed >= FIRST_FREE_SKILL_AT
            && (session.skill_points > 0 || session.gold >= skill_cost(&economy)));
    if !pressed {
        return;
    }
    if draft.kind == Some(DraftKind::Skill) && !draft.choices.is_empty() {
        if economy.pending_payment.is_none() {
            return;
        }
        session.paused = true;
        spawn_draft_ui(
            &mut commands,
            &mut draft,
            DraftKind::Skill,
            session.skill_rerolls,
        );
        return;
    }
    if draft.kind.is_some() {
        return;
    }
    let paid_cost = skill_cost(&economy);
    if session.skill_points == 0 && session.gold < paid_cost {
        return;
    }
    let choices = pick_skill_choices(
        &mut rng,
        session.level,
        session.elapsed,
        &build,
        &selected,
        &mut drought,
    );
    if choices.is_empty() {
        return;
    }
    let payment = if session.skill_points > 0 {
        session.skill_points -= 1;
        SkillDrawPayment::Free
    } else {
        session.gold -= paid_cost;
        economy.paid_draws = economy.paid_draws.saturating_add(1);
        SkillDrawPayment::Gold(paid_cost)
    };
    economy.pending_payment = Some(payment);
    open_draft(
        &mut commands,
        &mut session,
        &mut draft,
        DraftKind::Skill,
        choices,
    );
}

fn request_bond_draft(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    autoplay: Option<Res<Autoplay>>,
    interactions: Query<&Interaction, (With<BondDrawButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut rng: ResMut<RunRng>,
    selected: Res<SelectedHero>,
    mut economy: ResMut<BondEconomy>,
    mut drought: ResMut<DraftAntiDrought>,
    mut market_effects: ResMut<MarketEffects>,
) {
    if session.finished || session.paused || session.save_active || draft.root.is_some() {
        return;
    }
    let can_pay = economy.pending_payment.is_some()
        || economy.free_draw_available
        || session.wood >= bond_cost(&session);
    let automatic = autoplay.is_some()
        && session.elapsed >= FIRST_FREE_BOND_AT
        && session.elapsed >= session.next_auto_bond_at
        && can_pay;
    let pressed = keyboard.just_pressed(KeyCode::KeyF)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
        || automatic;
    if !pressed {
        return;
    }
    if session.elapsed < FIRST_FREE_BOND_AT {
        return;
    }
    if matches!(
        draft.kind,
        Some(DraftKind::Bond | DraftKind::BondReplacement)
    ) && !draft.choices.is_empty()
    {
        if economy.pending_payment.is_none() {
            return;
        }
        session.paused = true;
        let kind = draft.kind.expect("matched an open bond draft");
        spawn_draft_ui(&mut commands, &mut draft, kind, session.skill_rerolls);
        return;
    }
    if draft.kind.is_some() {
        return;
    }
    let cost = bond_cost(&session);
    if !economy.free_draw_available && session.wood < cost {
        return;
    }
    let compass_target = market_effects
        .bond_compass_available()
        .then(|| highest_progress_unfinished_bond(&build))
        .flatten();
    let choices = pick_bond_choices(&mut rng, &build, &selected, &mut drought, compass_target);
    if choices.is_empty() {
        return;
    }
    if compass_target.is_some_and(|target| choices.contains(&DraftChoice::Bond(target))) {
        let _ = market_effects.consume_bond_compass();
    }
    economy.pending_payment = Some(if economy.free_draw_available {
        economy.free_draw_available = false;
        BondDrawPayment::Free
    } else {
        session.wood -= cost;
        session.bond_draws = session.bond_draws.saturating_add(1);
        BondDrawPayment::Wood(cost)
    });
    if automatic {
        session.next_auto_bond_at = session.elapsed + 60.0;
    }
    open_draft(
        &mut commands,
        &mut session,
        &mut draft,
        DraftKind::Bond,
        choices,
    );
}

pub fn bond_cost(session: &GameSession) -> u32 {
    (10 + (session.bond_draws / 2) * 2).min(20)
}

fn open_draft(
    commands: &mut Commands,
    session: &mut GameSession,
    draft: &mut DraftState,
    kind: DraftKind,
    choices: Vec<DraftChoice>,
) {
    session.paused = true;
    draft.choices = choices;
    draft.kind = Some(kind);
    draft.pending_bond_replacement = None;
    draft.opened_for = 0.0;
    spawn_draft_ui(commands, draft, kind, session.skill_rerolls);
}

fn pick_skill_choices(
    rng: &mut RunRng,
    level: u32,
    elapsed: f32,
    build: &PlayerBuild,
    selected: &SelectedHero,
    drought: &mut DraftAntiDrought,
) -> Vec<DraftChoice> {
    let slots_full = active_skill_slots(build) >= SKILL_SLOT_LIMIT;
    let mut pool: Vec<SkillId> = (0..SKILLS.len())
        .map(|index| SkillId(index as u8))
        .filter(|id| {
            let def = id.definition();
            build.skill_rank(*id) < def.max_rank
                && (!slots_full || build.skill_rank(*id) > 0)
                && skill_rarity_available(def.rarity, level, elapsed)
        })
        .collect();
    if pool.is_empty() {
        return Vec::new();
    }
    pool.sort_by_key(|id| std::cmp::Reverse(build.skill_rank(*id)));
    let guaranteed = if build.skill_rank(pool[0]) > 0 {
        pool[0]
    } else {
        pool[rng.next_u32() as usize % pool.len().min(12)]
    };
    let mut choices = vec![guaranteed];
    let mut remaining: Vec<SkillId> = pool
        .iter()
        .copied()
        .filter(|candidate| *candidate != guaranteed)
        .collect();
    while choices.len() < 3 && !remaining.is_empty() {
        let index = rng.next_u32() as usize % remaining.len();
        let candidate = remaining.swap_remove(index);
        if rarity_fits_skill_draw(
            choices
                .iter()
                .any(|skill| skill.definition().rarity == Rarity::Ur),
            candidate.definition().rarity,
        ) {
            choices.push(candidate);
        }
    }

    let has_preferred = choices
        .iter()
        .any(|skill| tags_match_preference(skill.definition().tags, selected, build));
    if drought.skill_missed_preference && !has_preferred {
        let has_ur = choices
            .iter()
            .any(|skill| skill.definition().rarity == Rarity::Ur);
        let preferred: Vec<SkillId> = pool
            .iter()
            .copied()
            .filter(|skill| !choices.contains(skill))
            .filter(|skill| tags_match_preference(skill.definition().tags, selected, build))
            .filter(|skill| rarity_fits_skill_draw(has_ur, skill.definition().rarity))
            .collect();
        if !preferred.is_empty() {
            let replacement = choices.len().saturating_sub(1);
            choices[replacement] = preferred[rng.next_u32() as usize % preferred.len()];
        } else if has_ur {
            // A preferred UR can still replace the existing UR without violating
            // the one-UR-per-hand cap.
            let preferred_ur: Vec<SkillId> = pool
                .iter()
                .copied()
                .filter(|skill| !choices.contains(skill))
                .filter(|skill| skill.definition().rarity == Rarity::Ur)
                .filter(|skill| tags_match_preference(skill.definition().tags, selected, build))
                .collect();
            if !preferred_ur.is_empty() {
                let replacement = choices
                    .iter()
                    .position(|skill| skill.definition().rarity == Rarity::Ur)
                    .expect("has_ur was computed from choices");
                choices[replacement] = preferred_ur[rng.next_u32() as usize % preferred_ur.len()];
            }
        }
    }
    drought.skill_missed_preference = !choices
        .iter()
        .any(|skill| tags_match_preference(skill.definition().tags, selected, build));
    choices.into_iter().map(DraftChoice::Skill).collect()
}

fn skill_rarity_available(rarity: Rarity, level: u32, elapsed: f32) -> bool {
    match rarity {
        Rarity::R => true,
        Rarity::Sr => level >= 3,
        Rarity::Ssr => level >= 8,
        Rarity::Ur => elapsed >= EPIC_SKILL_UNLOCK_AT,
    }
}

fn rarity_fits_skill_draw(has_ur: bool, rarity: Rarity) -> bool {
    rarity != Rarity::Ur || !has_ur
}

fn active_skill_slots(build: &PlayerBuild) -> usize {
    build.skills.iter().filter(|rank| **rank > 0).count()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BondChoiceBucket {
    Existing,
    Preferred,
    Other,
}

fn pick_bond_choices(
    rng: &mut RunRng,
    build: &PlayerBuild,
    selected: &SelectedHero,
    drought: &mut DraftAntiDrought,
    guaranteed: Option<BondId>,
) -> Vec<DraftChoice> {
    let available = available_bonds(build);
    if available.is_empty() {
        return Vec::new();
    }

    let mut choices: Vec<BondId> = Vec::with_capacity(3);
    if let Some(guaranteed) = guaranteed.filter(|bond| available.contains(bond)) {
        choices.push(guaranteed);
    }
    if drought.bond_missed_preference {
        let preferred: Vec<BondId> = available
            .iter()
            .copied()
            .filter(|bond| !choices.contains(bond))
            .filter(|bond| bond_matches_preference(*bond, selected, build))
            .collect();
        if choices.len() < 3 && !preferred.is_empty() {
            choices.push(preferred[rng.next_u32() as usize % preferred.len()]);
        }
    }

    while choices.len() < 3 && choices.len() < available.len() {
        let remaining: Vec<BondId> = available
            .iter()
            .copied()
            .filter(|bond| !choices.contains(bond))
            .collect();
        let existing: Vec<BondId> = remaining
            .iter()
            .copied()
            .filter(|bond| build.bond(*bond) > 0)
            .collect();
        let preferred: Vec<BondId> = remaining
            .iter()
            .copied()
            .filter(|bond| build.bond(*bond) == 0)
            .filter(|bond| bond_matches_preference(*bond, selected, build))
            .collect();
        let other: Vec<BondId> = remaining
            .iter()
            .copied()
            .filter(|bond| build.bond(*bond) == 0)
            .filter(|bond| !bond_matches_preference(*bond, selected, build))
            .collect();
        let Some(bucket) = bond_bucket_for_roll(
            rng.next_u32(),
            !existing.is_empty(),
            !preferred.is_empty(),
            !other.is_empty(),
        ) else {
            break;
        };
        let candidates = match bucket {
            BondChoiceBucket::Existing => &existing,
            BondChoiceBucket::Preferred => &preferred,
            BondChoiceBucket::Other => &other,
        };
        choices.push(candidates[rng.next_u32() as usize % candidates.len()]);
    }

    drought.bond_missed_preference = !choices
        .iter()
        .any(|bond| bond_matches_preference(*bond, selected, build));
    choices.into_iter().map(DraftChoice::Bond).collect()
}

fn available_bonds(build: &PlayerBuild) -> Vec<BondId> {
    BondId::ALL
        .into_iter()
        .filter(|id| !build.bond_complete(*id) && build.bond(*id) < id.required())
        .collect()
}

fn highest_progress_unfinished_bond(build: &PlayerBuild) -> Option<BondId> {
    available_bonds(build)
        .into_iter()
        .max_by_key(|bond| (build.bond(*bond), std::cmp::Reverse(bond.index())))
}

fn bond_bucket_for_roll(
    roll: u32,
    has_existing: bool,
    has_preferred: bool,
    has_other: bool,
) -> Option<BondChoiceBucket> {
    let existing_weight = if has_existing { 45 } else { 0 };
    let preferred_weight = if has_preferred { 35 } else { 0 };
    let other_weight = if has_other { 20 } else { 0 };
    let total = existing_weight + preferred_weight + other_weight;
    if total == 0 {
        return None;
    }
    let roll = roll % total;
    if roll < existing_weight {
        Some(BondChoiceBucket::Existing)
    } else if roll < existing_weight + preferred_weight {
        Some(BondChoiceBucket::Preferred)
    } else {
        Some(BondChoiceBucket::Other)
    }
}

fn bond_matches_preference(bond: BondId, selected: &SelectedHero, build: &PlayerBuild) -> bool {
    tags_match_preference(BONDS[bond.index()].favored_tags, selected, build)
}

fn tags_match_preference(tags: &[Tag], selected: &SelectedHero, build: &PlayerBuild) -> bool {
    let hero_tags = HEROES[selected.kind.index()].tags;
    tags.iter().any(|tag| {
        hero_tags.contains(tag)
            || build
                .skills
                .iter()
                .enumerate()
                .any(|(index, rank)| *rank > 0 && SKILLS[index].tags.contains(tag))
    })
}

fn spawn_draft_ui(commands: &mut Commands, draft: &mut DraftState, kind: DraftKind, rerolls: u32) {
    let replacing = kind == DraftKind::BondReplacement;
    let replacement = draft.pending_bond_replacement;
    let heading = match (kind, replacement) {
        (DraftKind::Skill, _) => "CHOOSE A SKILL".to_string(),
        (DraftKind::Bond, _) => "CHOOSE A BOND".to_string(),
        (DraftKind::BondReplacement, Some(new_bond)) => {
            format!("REPLACE A BOND WITH {}", new_bond.title())
        }
        (DraftKind::BondReplacement, None) => "REPLACE A BOND".to_string(),
    };
    let root = commands
        .spawn((
            DraftRoot,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(if replacing { 8 } else { 18 }),
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.02, 0.045, 0.9)),
            GlobalZIndex(50),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(heading),
                TextFont {
                    font_size: FontSize::Px(if replacing { 26.0 } else { 34.0 }),
                    ..default()
                },
                TextColor(Color::srgb(0.75, 0.93, 1.0)),
            ));
            if replacing {
                parent.spawn((
                    Text::new("The draw is already paid. Choose one current family to remove."),
                    TextFont {
                        font_size: FontSize::Px(16.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.72, 0.8, 0.88)),
                    TextLayout::justify(Justify::Center),
                ));
            }
            parent
                .spawn(Node {
                    width: percent(if replacing {
                        REPLACEMENT_GRID_WIDTH_PERCENT
                    } else {
                        90.0
                    }),
                    max_width: px(if replacing {
                        REPLACEMENT_GRID_MAX_WIDTH
                    } else {
                        980.0
                    }),
                    height: px(if replacing {
                        REPLACEMENT_GRID_HEIGHT
                    } else {
                        230.0
                    }),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: if replacing {
                        FlexWrap::Wrap
                    } else {
                        FlexWrap::NoWrap
                    },
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Stretch,
                    column_gap: px(if replacing {
                        REPLACEMENT_COLUMN_GAP
                    } else {
                        16.0
                    }),
                    row_gap: px(if replacing { REPLACEMENT_ROW_GAP } else { 12.0 }),
                    ..default()
                })
                .with_children(|cards| {
                    for (index, choice) in draft.choices.iter().copied().enumerate() {
                        let (title, description) = match (kind, choice, replacement) {
                            (
                                DraftKind::BondReplacement,
                                DraftChoice::Bond(old_bond),
                                Some(new_bond),
                            ) => (
                                format!("{}  REPLACE {}", index + 1, old_bond.title()),
                                format!(
                                    "Remove all {} progress\n→ Add {} 1/{}",
                                    old_bond.title(),
                                    new_bond.title(),
                                    new_bond.required()
                                ),
                            ),
                            _ => (
                                format!("{}  {}", index + 1, choice.title()),
                                choice.description(),
                            ),
                        };
                        cards
                            .spawn((
                                Button,
                                DraftButton(index),
                                Node {
                                    width: percent(if replacing {
                                        REPLACEMENT_CARD_WIDTH_PERCENT
                                    } else {
                                        30.0
                                    }),
                                    min_width: px(if replacing {
                                        REPLACEMENT_CARD_MIN_WIDTH
                                    } else {
                                        210.0
                                    }),
                                    min_height: px(if replacing {
                                        REPLACEMENT_CARD_MIN_HEIGHT
                                    } else {
                                        0.0
                                    }),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    padding: UiRect::all(px(if replacing { 8 } else { 16 })),
                                    border: UiRect::all(px(2)),
                                    border_radius: BorderRadius::all(px(if replacing {
                                        12
                                    } else {
                                        16
                                    })),
                                    row_gap: px(if replacing { 6 } else { 14 }),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.055, 0.11, 0.18)),
                                BorderColor::all(choice_color(choice)),
                            ))
                            .with_children(|card| {
                                card.spawn((
                                    Text::new(title),
                                    TextFont {
                                        font_size: FontSize::Px(if replacing {
                                            16.0
                                        } else {
                                            23.0
                                        }),
                                        ..default()
                                    },
                                    TextColor(Color::WHITE),
                                ));
                                card.spawn((
                                    Text::new(description),
                                    TextFont {
                                        font_size: FontSize::Px(if replacing {
                                            13.0
                                        } else {
                                            17.0
                                        }),
                                        ..default()
                                    },
                                    TextColor(Color::srgb(0.72, 0.8, 0.88)),
                                    TextLayout::justify(Justify::Center),
                                ));
                            });
                    }
                });
            parent
                .spawn(Node {
                    height: px(if replacing { 44 } else { 48 }),
                    flex_direction: FlexDirection::Row,
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|actions| {
                    if !replacing {
                        spawn_draft_action(
                            actions,
                            DraftAction::Refresh,
                            format!("REFRESH ({rerolls})"),
                        );
                    }
                    spawn_draft_action(actions, DraftAction::Abandon, "ABANDON (+1)".to_string());
                    spawn_draft_action(actions, DraftAction::Later, "CHOOSE LATER".to_string());
                });
        })
        .id();
    draft.root = Some(root);
}

fn spawn_draft_action(parent: &mut ChildSpawnerCommands, action: DraftAction, label: String) {
    parent
        .spawn((
            Button,
            DraftActionButton(action),
            Node {
                min_width: px(150),
                height: px(42),
                padding: UiRect::horizontal(px(14)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.10, 0.25, 0.20)),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont {
                    font_size: FontSize::Px(14.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
        });
}

fn choice_color(choice: DraftChoice) -> Color {
    match choice {
        DraftChoice::Bond(_) => Color::srgb(0.95, 0.72, 0.2),
        DraftChoice::Skill(skill) => match skill.definition().rarity {
            Rarity::R => Color::srgb(0.18, 0.72, 1.0),
            Rarity::Sr => Color::srgb(0.34, 0.95, 0.58),
            Rarity::Ssr => Color::srgb(0.98, 0.66, 0.12),
            Rarity::Ur => Color::srgb(1.0, 0.25, 0.35),
        },
    }
}

fn handle_draft_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &DraftButton), Changed<Interaction>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut skill_economy: ResMut<SkillEconomy>,
    mut bond_economy: ResMut<BondEconomy>,
) {
    for (interaction, button) in &interactions {
        if *interaction == Interaction::Pressed {
            choose(
                button.0,
                &mut commands,
                &mut session,
                &mut build,
                &mut draft,
                &mut skill_economy,
                &mut bond_economy,
            );
            break;
        }
    }
}

fn handle_draft_actions(
    mut commands: Commands,
    interactions: Query<(&Interaction, &DraftActionButton), Changed<Interaction>>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut skill_economy: ResMut<SkillEconomy>,
    mut bond_economy: ResMut<BondEconomy>,
    mut rng: ResMut<RunRng>,
    selected: Res<SelectedHero>,
    mut drought: ResMut<DraftAntiDrought>,
) {
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        let Some(kind) = draft.kind else {
            return;
        };
        match button.0 {
            DraftAction::Refresh => {
                if session.skill_rerolls == 0 {
                    return;
                }
                session.skill_rerolls -= 1;
                draft.choices = match kind {
                    DraftKind::Skill => pick_skill_choices(
                        &mut rng,
                        session.level,
                        session.elapsed,
                        &build,
                        &selected,
                        &mut drought,
                    ),
                    DraftKind::Bond => {
                        pick_bond_choices(&mut rng, &build, &selected, &mut drought, None)
                    }
                    DraftKind::BondReplacement => return,
                };
                if let Some(root) = draft.root.take() {
                    commands.entity(root).despawn();
                }
                spawn_draft_ui(&mut commands, &mut draft, kind, session.skill_rerolls);
            }
            DraftAction::Abandon => {
                session.skill_rerolls = session.skill_rerolls.saturating_add(1);
                clear_draft(
                    &mut commands,
                    &mut session,
                    &mut draft,
                    &mut skill_economy,
                    &mut bond_economy,
                );
            }
            DraftAction::Later => {
                if let Some(root) = draft.root.take() {
                    commands.entity(root).despawn();
                }
                draft.opened_for = 0.0;
                session.paused = false;
            }
        }
        break;
    }
}

fn handle_draft_keys(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut skill_economy: ResMut<SkillEconomy>,
    mut bond_economy: ResMut<BondEconomy>,
) {
    if draft.root.is_none() {
        return;
    }
    let choice = if keyboard.just_pressed(KeyCode::Digit1) {
        Some(0)
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        Some(1)
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        Some(2)
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        Some(3)
    } else if keyboard.just_pressed(KeyCode::Digit5) {
        Some(4)
    } else if keyboard.just_pressed(KeyCode::Digit6) {
        Some(5)
    } else if keyboard.just_pressed(KeyCode::Digit7) {
        Some(6)
    } else {
        None
    };
    if let Some(index) = choice {
        choose(
            index,
            &mut commands,
            &mut session,
            &mut build,
            &mut draft,
            &mut skill_economy,
            &mut bond_economy,
        );
    }
}

fn autoplay_draft(
    mut commands: Commands,
    time: Res<Time>,
    autoplay: Option<Res<Autoplay>>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut skill_economy: ResMut<SkillEconomy>,
    mut bond_economy: ResMut<BondEconomy>,
) {
    if autoplay.is_none() || draft.root.is_none() {
        return;
    }
    draft.opened_for += time.delta_secs();
    if draft.opened_for >= 1.15 {
        let index = if draft.kind == Some(DraftKind::BondReplacement) {
            draft
                .choices
                .iter()
                .enumerate()
                .filter_map(|(index, choice)| match choice {
                    DraftChoice::Bond(bond) => Some((index, *bond)),
                    DraftChoice::Skill(_) => None,
                })
                .min_by_key(|(_, bond)| (build.bond(*bond), bond.index()))
                .map(|(index, _)| index)
                .unwrap_or(0)
        } else {
            session.level as usize % draft.choices.len().max(1)
        };
        choose(
            index,
            &mut commands,
            &mut session,
            &mut build,
            &mut draft,
            &mut skill_economy,
            &mut bond_economy,
        );
    }
}

fn choose(
    index: usize,
    commands: &mut Commands,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    draft: &mut DraftState,
    skill_economy: &mut SkillEconomy,
    bond_economy: &mut BondEconomy,
) {
    let Some(choice) = draft.choices.get(index).copied() else {
        return;
    };
    match choice {
        DraftChoice::Skill(skill) => {
            if skill_economy.pending_payment.is_none() {
                return;
            }
            // Full bars intentionally offer upgrades only. Replacement needs an
            // additional target-selection UI and is outside this first ruleset.
            if build.skill_rank(skill) == 0 && active_skill_slots(build) >= SKILL_SLOT_LIMIT {
                return;
            }
            apply_skill(skill, session, build);
        }
        DraftChoice::Bond(bond) => {
            if bond_economy.pending_payment.is_none() {
                return;
            }
            if draft.kind == Some(DraftKind::BondReplacement) {
                let Some(new_bond) = draft.pending_bond_replacement else {
                    return;
                };
                if build.bond(bond) == 0
                    || build.bond_complete(bond)
                    || build.bond(new_bond) > 0
                    || build.bond_complete(new_bond)
                {
                    return;
                }
                replace_bond(bond, new_bond, session, build);
                clear_draft(commands, session, draft, skill_economy, bond_economy);
                return;
            }
            if build.bond(bond) == 0 && build.active_bond_count() >= BOND_SLOT_LIMIT {
                let rerolls = session.skill_rerolls;
                begin_bond_replacement(bond, commands, session, build, draft, rerolls);
                return;
            }
            apply_bond(bond, session, build);
        }
    }
    clear_draft(commands, session, draft, skill_economy, bond_economy);
}

fn clear_draft(
    commands: &mut Commands,
    session: &mut GameSession,
    draft: &mut DraftState,
    skill_economy: &mut SkillEconomy,
    bond_economy: &mut BondEconomy,
) {
    if let Some(root) = draft.root.take() {
        commands.entity(root).despawn();
    }
    draft.choices.clear();
    draft.kind = None;
    draft.pending_bond_replacement = None;
    draft.opened_for = 0.0;
    skill_economy.pending_payment = None;
    bond_economy.pending_payment = None;
    session.paused = false;
}

fn begin_bond_replacement(
    new_bond: BondId,
    commands: &mut Commands,
    session: &mut GameSession,
    build: &PlayerBuild,
    draft: &mut DraftState,
    rerolls: u32,
) {
    let choices: Vec<DraftChoice> = BondId::ALL
        .into_iter()
        .filter(|bond| build.bond(*bond) > 0 && !build.bond_complete(*bond))
        .take(BOND_SLOT_LIMIT)
        .map(DraftChoice::Bond)
        .collect();
    if choices.is_empty() {
        return;
    }
    if let Some(root) = draft.root.take() {
        commands.entity(root).despawn();
    }
    session.paused = true;
    draft.choices = choices;
    draft.kind = Some(DraftKind::BondReplacement);
    draft.pending_bond_replacement = Some(new_bond);
    draft.opened_for = 0.0;
    spawn_draft_ui(commands, draft, DraftKind::BondReplacement, rerolls);
}

fn replace_bond(
    old_bond: BondId,
    new_bond: BondId,
    session: &mut GameSession,
    build: &mut PlayerBuild,
) {
    remove_bond(old_bond, session, build);
    build.bonds[new_bond.index()] = 0;
    build.completed_bonds[new_bond.index()] = false;
    apply_bond(new_bond, session, build);
}

fn remove_bond(bond: BondId, session: &mut GameSession, build: &mut PlayerBuild) {
    let old_rank = build.bond(bond);
    build.bonds[bond.index()] = 0;
    build.completed_bonds[bond.index()] = false;

    // Bastion 2 is the sole bond tier whose benefit is materialized directly
    // in GameSession instead of being derived live by bonds.rs.
    if bond == BondId::Bastion && old_rank >= 2 {
        session.core_max_hp = (session.core_max_hp - 250.0).max(1.0);
        session.core_hp = session.core_hp.min(session.core_max_hp);
    }
}

fn apply_skill(skill: SkillId, session: &mut GameSession, build: &mut PlayerBuild) {
    let def = skill.definition();
    let rank = (build.skills[skill.0 as usize] + 1).min(def.max_rank);
    build.skills[skill.0 as usize] = rank;

    match def.effect.kind {
        SkillEffectKind::Projectile | SkillEffectKind::AttackProc => {
            build.attack_damage += 2.5;
            build.projectile_speed *= 1.025;
        }
        SkillEffectKind::Nova | SkillEffectKind::GroundZone => {
            build.attack_damage += 3.5;
        }
        SkillEffectKind::Orbit | SkillEffectKind::Echo => {
            build.attacks_per_second *= 1.025;
        }
        SkillEffectKind::Summon => {
            if rank == 1 || rank == 4 {
                build.multishot = build.multishot.saturating_add(1).min(7);
            }
        }
        SkillEffectKind::Mark => build.pierce = build.pierce.saturating_add(1).min(12),
        SkillEffectKind::Revive => {
            session.core_max_hp += 80.0;
            session.core_hp = (session.core_hp + 180.0).min(session.core_max_hp);
        }
    }
    if def.tags.contains(&Tag::Storm) {
        build.attacks_per_second *= 1.018;
    }
    if def.tags.contains(&Tag::Flame) {
        build.attack_damage += 2.0;
    }
    if def.tags.contains(&Tag::Guard) {
        session.core_hp = (session.core_hp + 35.0).min(session.core_max_hp);
    }
}

fn apply_bond(bond: BondId, session: &mut GameSession, build: &mut PlayerBuild) {
    let required = bond.required();
    let rank = (build.bonds[bond.index()] + 1).min(required);
    build.bonds[bond.index()] = rank;
    let thresholds = [2, 4, 6];
    let tier = if rank == thresholds[0] {
        2
    } else if rank == thresholds[1] {
        4
    } else if rank == thresholds[2] {
        6
    } else {
        rank
    };
    if let (BondId::Bastion, 2) = (bond, tier) {
        session.core_max_hp += 250.0;
        session.core_hp += 250.0;
    }
    if rank >= required {
        build.completed_bonds[bond.index()] = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<DraftState>()
            .init_resource::<RunRng>()
            .init_resource::<SelectedHero>()
            .init_resource::<MarketEffects>()
            .add_plugins(ProgressionPlugin);
        app.update();
        app
    }

    #[test]
    fn bond_selection_only_records_progress_and_never_reapplies_generic_stats() {
        for bond in BondId::ALL {
            let mut session = GameSession::default();
            let mut build = PlayerBuild::default();
            let baseline = (
                build.attack_damage,
                build.attacks_per_second,
                build.projectile_speed,
                build.move_speed,
                build.multishot,
                build.pierce,
                session.wood,
                session.core_hp,
                session.core_max_hp,
            );
            for _ in 0..6 {
                apply_bond(bond, &mut session, &mut build);
            }
            let expected_core = if bond == BondId::Bastion {
                (baseline.7 + 250.0, baseline.8 + 250.0)
            } else {
                (baseline.7, baseline.8)
            };
            assert_eq!(
                (
                    build.attack_damage,
                    build.attacks_per_second,
                    build.projectile_speed,
                    build.move_speed,
                    build.multishot,
                    build.pierce,
                    session.wood,
                ),
                (
                    baseline.0, baseline.1, baseline.2, baseline.3, baseline.4, baseline.5,
                    baseline.6,
                ),
                "{bond:?} kept an obsolete generic stat bonus",
            );
            assert_eq!((session.core_hp, session.core_max_hp), expected_core);
            assert!(build.bond_complete(bond));
        }
    }

    fn tap_g(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyG);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(KeyCode::KeyG);
        input.clear();
    }

    fn tap_f(app: &mut App) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyF);
        app.update();
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.release(KeyCode::KeyF);
        input.clear();
    }

    fn press_action(app: &mut App, wanted: DraftAction) {
        let entity = {
            let world = app.world_mut();
            let mut actions = world.query::<(Entity, &DraftActionButton)>();
            actions
                .iter(world)
                .find_map(|(entity, action)| (action.0 == wanted).then_some(entity))
                .expect("requested draft action")
        };
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
    }

    fn press_first_choice(app: &mut App) {
        press_choice(app, 0);
    }

    fn press_choice(app: &mut App, wanted: usize) {
        let entity = {
            let world = app.world_mut();
            let mut choices = world.query::<(Entity, &DraftButton)>();
            choices
                .iter(world)
                .find_map(|(entity, choice)| (choice.0 == wanted).then_some(entity))
                .expect("requested draft choice")
        };
        app.world_mut()
            .entity_mut(entity)
            .insert(Interaction::Pressed);
        app.update();
    }

    fn install_test_bond_draft(
        app: &mut App,
        choices: Vec<BondId>,
        replacement: Option<BondId>,
        payment: BondDrawPayment,
    ) {
        let root = app.world_mut().spawn(DraftRoot).id();
        for (index, _) in choices.iter().enumerate() {
            let button = app
                .world_mut()
                .spawn((DraftButton(index), Interaction::None))
                .id();
            app.world_mut().entity_mut(root).add_child(button);
        }
        for action in [
            DraftAction::Refresh,
            DraftAction::Abandon,
            DraftAction::Later,
        ] {
            let button = app
                .world_mut()
                .spawn((DraftActionButton(action), Interaction::None))
                .id();
            app.world_mut().entity_mut(root).add_child(button);
        }
        {
            let mut draft = app.world_mut().resource_mut::<DraftState>();
            draft.choices = choices.into_iter().map(DraftChoice::Bond).collect();
            draft.root = Some(root);
            draft.opened_for = 0.0;
            draft.kind = Some(if replacement.is_some() {
                DraftKind::BondReplacement
            } else {
                DraftKind::Bond
            });
            draft.pending_bond_replacement = replacement;
        }
        app.world_mut()
            .resource_mut::<BondEconomy>()
            .pending_payment = Some(payment);
        app.world_mut().resource_mut::<GameSession>().paused = true;
    }

    fn finish_autoplay_draft_after_1_15_seconds(app: &mut App) {
        let mut draft = app.world_mut().resource_mut::<DraftState>();
        assert!(draft.root.is_some(), "autoplay needs an open draft");
        draft.opened_for = 1.15;
        app.update();
    }

    #[test]
    fn grants_only_first_time_and_milestone_skill_points() {
        let mut app = test_app();
        assert_eq!(FIRST_FREE_SKILL_AT, 4.0);
        assert_eq!(app.world().resource::<GameSession>().skill_points, 0);

        app.world_mut().resource_mut::<GameSession>().elapsed = 3.999;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().skill_points, 0);

        app.world_mut().resource_mut::<GameSession>().elapsed = 4.0;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().skill_points, 1);
        app.update();
        assert_eq!(app.world().resource::<GameSession>().skill_points, 1);

        app.world_mut().resource_mut::<GameSession>().level = 5;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().skill_points, 2);

        app.world_mut().resource_mut::<GameSession>().level = 9;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().skill_points, 2);

        for (level, expected) in [(10, 3), (15, 4), (20, 5), (25, 6)] {
            app.world_mut().resource_mut::<GameSession>().level = level;
            app.update();
            assert_eq!(app.world().resource::<GameSession>().skill_points, expected);
        }
        let economy = app.world().resource::<SkillEconomy>();
        assert!(economy.first_free_granted);
        assert_eq!(economy.level_grants, 0b1_1111);
    }

    #[test]
    fn autoplay_at_four_seconds_spends_the_free_draw_before_gold() {
        let mut app = test_app();
        app.insert_resource(Autoplay);
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 4.0;
            session.gold = 500;
        }

        app.update();

        let session = app.world().resource::<GameSession>();
        assert_eq!(session.gold, 500);
        assert_eq!(session.skill_points, 0);
        assert!(session.paused);
        let economy = app.world().resource::<SkillEconomy>();
        assert_eq!(economy.paid_draws, 0);
        assert_eq!(economy.pending_payment, Some(SkillDrawPayment::Free));
        assert_eq!(
            app.world().resource::<DraftState>().kind,
            Some(DraftKind::Skill)
        );
    }

    #[test]
    fn autoplay_finishes_free_then_pays_eighty_and_settles_each_draft_once() {
        let mut app = test_app();
        app.insert_resource(Autoplay);
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 4.0;
            session.gold = 80;
        }

        app.update();
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Free)
        );
        finish_autoplay_draft_after_1_15_seconds(&mut app);
        assert!(
            app.world()
                .resource::<SkillEconomy>()
                .pending_payment
                .is_none()
        );

        app.update();
        let session = app.world().resource::<GameSession>();
        assert_eq!(session.gold, 0);
        assert!(session.paused);
        let economy = app.world().resource::<SkillEconomy>();
        assert_eq!(economy.paid_draws, 1);
        assert_eq!(economy.pending_payment, Some(SkillDrawPayment::Gold(80)));

        finish_autoplay_draft_after_1_15_seconds(&mut app);
        app.update();
        app.update();
        let economy = app.world().resource::<SkillEconomy>();
        assert_eq!(economy.paid_draws, 1);
        assert!(economy.pending_payment.is_none());
        assert_eq!(app.world().resource::<GameSession>().gold, 0);
        assert!(app.world().resource::<DraftState>().kind.is_none());
    }

    #[test]
    fn autoplay_obeys_eighty_then_ninety_five_costs_and_later_never_recharges() {
        let mut app = test_app();
        app.insert_resource(Autoplay);
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 4.0;
            session.gold = 79;
        }

        app.update();
        finish_autoplay_draft_after_1_15_seconds(&mut app);
        app.update();
        assert_eq!(app.world().resource::<GameSession>().gold, 79);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 0);
        assert!(app.world().resource::<DraftState>().kind.is_none());

        app.world_mut().resource_mut::<GameSession>().gold = 80;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().gold, 0);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 1);
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(80))
        );
        finish_autoplay_draft_after_1_15_seconds(&mut app);

        app.world_mut().resource_mut::<GameSession>().gold = 95;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().gold, 0);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(95))
        );

        press_action(&mut app, DraftAction::Later);
        assert!(!app.world().resource::<GameSession>().paused);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_eq!(app.world().resource::<GameSession>().gold, 0);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(95))
        );

        tap_g(&mut app);
        assert!(app.world().resource::<GameSession>().paused);
        assert!(app.world().resource::<DraftState>().root.is_some());
        assert_eq!(app.world().resource::<GameSession>().gold, 0);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(95))
        );

        finish_autoplay_draft_after_1_15_seconds(&mut app);
        app.update();
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
        assert!(
            app.world()
                .resource::<SkillEconomy>()
                .pending_payment
                .is_none()
        );
        assert_eq!(app.world().resource::<GameSession>().gold, 0);
    }

    #[test]
    fn skill_price_increases_and_caps() {
        let mut economy = SkillEconomy::default();
        assert_eq!(skill_cost(&economy), 80);
        economy.paid_draws = 1;
        assert_eq!(skill_cost(&economy), 95);
        economy.paid_draws = 12;
        assert_eq!(skill_cost(&economy), 260);
        economy.paid_draws = u32::MAX;
        assert_eq!(skill_cost(&economy), 260);
    }

    #[test]
    fn ur_skills_unlock_at_twelve_minutes_and_are_capped_per_hand() {
        assert!(!skill_rarity_available(Rarity::Ur, 30, 719.999));
        assert!(skill_rarity_available(Rarity::Ur, 1, 720.0));
        assert!(rarity_fits_skill_draw(false, Rarity::Ur));
        assert!(!rarity_fits_skill_draw(true, Rarity::Ur));
        assert!(rarity_fits_skill_draw(true, Rarity::Ssr));
    }

    #[test]
    fn bond_weight_boundaries_are_exactly_45_35_20() {
        for roll in 0..45 {
            assert_eq!(
                bond_bucket_for_roll(roll, true, true, true),
                Some(BondChoiceBucket::Existing)
            );
        }
        for roll in 45..80 {
            assert_eq!(
                bond_bucket_for_roll(roll, true, true, true),
                Some(BondChoiceBucket::Preferred)
            );
        }
        for roll in 80..100 {
            assert_eq!(
                bond_bucket_for_roll(roll, true, true, true),
                Some(BondChoiceBucket::Other)
            );
        }
        assert_eq!(
            bond_bucket_for_roll(100, true, true, true),
            Some(BondChoiceBucket::Existing)
        );
        assert_eq!(bond_bucket_for_roll(0, false, false, false), None);
    }

    #[test]
    fn anti_drought_forces_a_preferred_card_on_the_next_hand() {
        let mut build = PlayerBuild::default();
        build.skills.fill(0);
        let selected = SelectedHero {
            kind: super::super::HeroKind::StarWagerer,
            locked: true,
        };

        let mut rng = RunRng(0x0A11_CE55);
        let mut drought = DraftAntiDrought {
            skill_missed_preference: true,
            bond_missed_preference: true,
        };
        let skills = pick_skill_choices(
            &mut rng,
            30,
            EPIC_SKILL_UNLOCK_AT,
            &build,
            &selected,
            &mut drought,
        );
        assert!(skills.iter().any(|choice| {
            matches!(choice, DraftChoice::Skill(skill)
                if tags_match_preference(skill.definition().tags, &selected, &build))
        }));
        assert!(!drought.skill_missed_preference);

        let bonds = pick_bond_choices(&mut rng, &build, &selected, &mut drought, None);
        assert!(bonds.iter().any(|choice| {
            matches!(choice, DraftChoice::Bond(bond)
                if bond_matches_preference(*bond, &selected, &build))
        }));
        assert!(!drought.bond_missed_preference);
    }

    #[test]
    fn seven_full_bond_slots_can_still_roll_a_new_family() {
        let mut build = PlayerBuild::default();
        for bond in BondId::ALL.into_iter().take(7) {
            build.bonds[bond.index()] = 1;
        }
        let selected = SelectedHero::default();
        let mut rng = RunRng::default();
        let mut drought = DraftAntiDrought {
            bond_missed_preference: true,
            ..default()
        };
        assert_eq!(available_bonds(&build).len(), BondId::COUNT);
        let mut saw_new_family = false;
        for _ in 0..40 {
            let choices = pick_bond_choices(&mut rng, &build, &selected, &mut drought, None);
            assert_eq!(choices.len(), 3);
            saw_new_family |= choices
                .into_iter()
                .any(|choice| matches!(choice, DraftChoice::Bond(bond) if build.bond(bond) == 0));
        }
        assert!(saw_new_family, "full slots must not suppress new families");
    }

    #[test]
    fn new_family_at_seven_slots_waits_for_an_explicit_replacement() {
        let mut app = test_app();
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            for bond in BondId::ALL.into_iter().take(BOND_SLOT_LIMIT) {
                build.bonds[bond.index()] = 1;
            }
            build.bonds[BondId::Bastion.index()] = BondId::Bastion.required();
            build.completed_bonds[BondId::Bastion.index()] = true;
        }
        let new_bond = BondId::Blood;
        install_test_bond_draft(&mut app, vec![new_bond], None, BondDrawPayment::Wood(10));
        let first_root = app
            .world()
            .resource::<DraftState>()
            .root
            .expect("first-stage root");

        press_choice(&mut app, 0);

        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.bond(new_bond), 0, "new family applied too early");
        assert_eq!(build.active_bond_count(), 7);
        let draft = app.world().resource::<DraftState>();
        assert_eq!(draft.kind, Some(DraftKind::BondReplacement));
        assert_eq!(draft.pending_bond_replacement, Some(new_bond));
        assert_eq!(draft.choices.len(), BOND_SLOT_LIMIT);
        assert!(draft.choices.iter().all(|choice| {
            matches!(choice, DraftChoice::Bond(bond)
                if build.bond(*bond) > 0 && !build.bond_complete(*bond))
        }));
        assert!(draft.root.is_some());
        assert_ne!(draft.root, Some(first_root));
        assert!(build.bond_complete(BondId::Bastion));
        assert!(app.world().resource::<GameSession>().paused);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Wood(10))
        );
    }

    #[test]
    fn replacement_clears_exactly_one_family_then_adds_the_new_family_at_one() {
        let mut app = test_app();
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.bonds[BondId::Thunder.index()] = 3;
            build.bonds[BondId::Ember.index()] = 2;
            build.bonds[BondId::Spirits.index()] = 4;
            build.bonds[BondId::Bastion.index()] = BondId::Bastion.required();
            build.completed_bonds[BondId::Bastion.index()] = true;
        }
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.core_max_hp += 250.0;
            session.core_hp += 250.0;
        }
        let before = app.world().resource::<PlayerBuild>().bonds;
        let completed_before = app.world().resource::<PlayerBuild>().completed_bonds;
        let core_before = {
            let session = app.world().resource::<GameSession>();
            (session.core_hp, session.core_max_hp)
        };
        install_test_bond_draft(
            &mut app,
            vec![BondId::Thunder, BondId::Ember, BondId::Spirits],
            Some(BondId::Blood),
            BondDrawPayment::Free,
        );

        press_choice(&mut app, 1);

        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.bond(BondId::Ember), 0);
        assert!(!build.bond_complete(BondId::Ember));
        assert_eq!(build.bond(BondId::Blood), 1);
        for bond in BondId::ALL {
            if !matches!(bond, BondId::Ember | BondId::Blood) {
                assert_eq!(build.bond(bond), before[bond.index()], "changed {bond:?}");
                assert_eq!(
                    build.bond_complete(bond),
                    completed_before[bond.index()],
                    "changed completed state for {bond:?}"
                );
            }
        }
        assert_eq!(build.active_bond_count(), 3);
        let session = app.world().resource::<GameSession>();
        assert_eq!((session.core_hp, session.core_max_hp), core_before);
        assert!(app.world().resource::<DraftState>().kind.is_none());
        assert!(!app.world().resource::<GameSession>().paused);
        assert!(
            app.world()
                .resource::<BondEconomy>()
                .pending_payment
                .is_none()
        );
    }

    #[test]
    fn abandoning_replacement_grants_exactly_one_refresh() {
        let mut app = test_app();
        app.world_mut().resource_mut::<GameSession>().skill_rerolls = 2;
        app.world_mut().resource_mut::<PlayerBuild>().bonds[BondId::Thunder.index()] = 3;
        install_test_bond_draft(
            &mut app,
            vec![BondId::Thunder],
            Some(BondId::Blood),
            BondDrawPayment::Free,
        );

        press_action(&mut app, DraftAction::Abandon);
        app.update();

        let session = app.world().resource::<GameSession>();
        assert_eq!(session.skill_rerolls, 3);
        assert!(!session.paused);
        let draft = app.world().resource::<DraftState>();
        assert!(draft.kind.is_none());
        assert!(draft.pending_bond_replacement.is_none());
        assert!(draft.root.is_none());
        assert!(
            app.world()
                .resource::<BondEconomy>()
                .pending_payment
                .is_none()
        );
    }

    #[test]
    fn autoplay_replaces_lowest_progress_then_lowest_bond_id_without_stalling() {
        let mut app = test_app();
        app.insert_resource(Autoplay);
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.bonds[BondId::Thunder.index()] = 1;
            build.bonds[BondId::Ember.index()] = 1;
            build.bonds[BondId::Spirits.index()] = 2;
            build.bonds[BondId::Elements.index()] = 3;
            build.bonds[BondId::Arcane.index()] = 4;
            build.bonds[BondId::Triune.index()] = 5;
            build.bonds[BondId::Ascension.index()] = 2;
        }
        install_test_bond_draft(
            &mut app,
            vec![
                BondId::Spirits,
                BondId::Ember,
                BondId::Ascension,
                BondId::Thunder,
                BondId::Elements,
                BondId::Arcane,
                BondId::Triune,
            ],
            Some(BondId::Blood),
            BondDrawPayment::Free,
        );
        app.world_mut().resource_mut::<DraftState>().opened_for = 1.15;

        app.update();

        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.bond(BondId::Thunder), 0);
        assert_eq!(build.bond(BondId::Ember), 1);
        assert_eq!(build.bond(BondId::Blood), 1);
        assert!(app.world().resource::<DraftState>().kind.is_none());
        assert!(!app.world().resource::<GameSession>().paused);
    }

    #[test]
    fn completed_bond_releases_a_slot_and_new_family_applies_directly() {
        let mut app = test_app();
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.bonds[BondId::Thunder.index()] = BondId::Thunder.required();
            build.completed_bonds[BondId::Thunder.index()] = true;
            for bond in [
                BondId::Ember,
                BondId::Spirits,
                BondId::Elements,
                BondId::Arcane,
                BondId::Triune,
                BondId::Ascension,
            ] {
                build.bonds[bond.index()] = 1;
            }
            assert_eq!(build.active_bond_count(), 6);
        }
        install_test_bond_draft(&mut app, vec![BondId::Blood], None, BondDrawPayment::Free);

        press_choice(&mut app, 0);

        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.bond(BondId::Blood), 1);
        assert_eq!(build.active_bond_count(), 7);
        assert!(build.bond_complete(BondId::Thunder));
        let draft = app.world().resource::<DraftState>();
        assert!(draft.kind.is_none());
        assert!(draft.pending_bond_replacement.is_none());
    }

    #[test]
    fn existing_family_upgrade_stays_direct_when_all_seven_slots_are_full() {
        let mut app = test_app();
        for bond in BondId::ALL.into_iter().take(BOND_SLOT_LIMIT) {
            app.world_mut().resource_mut::<PlayerBuild>().bonds[bond.index()] = 1;
        }
        app.world_mut().resource_mut::<PlayerBuild>().bonds[BondId::Thunder.index()] = 2;
        install_test_bond_draft(&mut app, vec![BondId::Thunder], None, BondDrawPayment::Free);

        press_choice(&mut app, 0);

        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.bond(BondId::Thunder), 3);
        assert_eq!(build.active_bond_count(), 7);
        let draft = app.world().resource::<DraftState>();
        assert!(draft.kind.is_none());
        assert!(draft.pending_bond_replacement.is_none());
    }

    #[test]
    fn replacement_grid_fits_capture_and_compact_landscape_phone_sizes() {
        fn fits(width: f32, height: f32) -> bool {
            let grid_width =
                (width * REPLACEMENT_GRID_WIDTH_PERCENT / 100.0).min(REPLACEMENT_GRID_MAX_WIDTH);
            let card_width = (grid_width * REPLACEMENT_CARD_WIDTH_PERCENT / 100.0)
                .max(REPLACEMENT_CARD_MIN_WIDTH);
            let three_cards = card_width * 3.0 + REPLACEMENT_COLUMN_GAP * 2.0;
            let three_rows = REPLACEMENT_CARD_MIN_HEIGHT * 3.0 + REPLACEMENT_ROW_GAP * 2.0;
            // 26px heading + 16px helper copy use a conservative 1.2 line height.
            let vertical = REPLACEMENT_GRID_HEIGHT + 26.0 * 1.2 + 16.0 * 1.2 + 44.0 + 8.0 * 3.0;
            three_cards <= grid_width && three_rows <= REPLACEMENT_GRID_HEIGHT && vertical <= height
        }

        assert!(fits(1280.0, 720.0));
        assert!(fits(960.0, 540.0));
        assert!(fits(800.0, 450.0));
    }

    #[test]
    fn replacement_choose_later_reopens_without_a_second_payment() {
        let mut app = test_app();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = FIRST_FREE_BOND_AT;
            session.wood = 90;
            session.bond_draws = 1;
        }
        {
            let mut economy = app.world_mut().resource_mut::<BondEconomy>();
            economy.first_free_granted = true;
            economy.free_draw_available = false;
        }
        app.world_mut().resource_mut::<PlayerBuild>().bonds[BondId::Thunder.index()] = 2;
        install_test_bond_draft(
            &mut app,
            vec![BondId::Thunder],
            Some(BondId::Blood),
            BondDrawPayment::Wood(10),
        );

        press_action(&mut app, DraftAction::Later);
        assert!(!app.world().resource::<GameSession>().paused);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_eq!(app.world().resource::<GameSession>().wood, 90);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 1);

        tap_f(&mut app);

        assert!(app.world().resource::<GameSession>().paused);
        assert!(app.world().resource::<DraftState>().root.is_some());
        assert_eq!(
            app.world().resource::<DraftState>().kind,
            Some(DraftKind::BondReplacement)
        );
        assert_eq!(app.world().resource::<GameSession>().wood, 90);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 1);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Wood(10))
        );

        press_choice(&mut app, 0);
        assert_eq!(app.world().resource::<GameSession>().wood, 90);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 1);
        assert_eq!(app.world().resource::<PlayerBuild>().bond(BondId::Blood), 1);
    }

    #[test]
    fn bond_compass_guarantees_the_highest_progress_unfinished_bond() {
        let mut build = PlayerBuild::default();
        build.bonds[BondId::Thunder.index()] = 2;
        build.bonds[BondId::Ember.index()] = 4;
        build.bonds[BondId::Spirits.index()] = 1;
        let selected = SelectedHero::default();
        let mut rng = RunRng::default();
        let mut drought = DraftAntiDrought::default();

        let target = highest_progress_unfinished_bond(&build);
        assert_eq!(target, Some(BondId::Ember));
        let choices = pick_bond_choices(&mut rng, &build, &selected, &mut drought, target);
        assert!(choices.contains(&DraftChoice::Bond(BondId::Ember)));
    }

    #[test]
    fn free_bond_draw_is_banked_and_pending_actions_never_double_charge() {
        let mut app = test_app();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = FIRST_FREE_BOND_AT - 0.1;
            session.wood = 100;
        }
        app.update();
        tap_f(&mut app);
        assert!(app.world().resource::<DraftState>().kind.is_none());
        assert!(!app.world().resource::<BondEconomy>().free_draw_available);

        app.world_mut().resource_mut::<GameSession>().elapsed = FIRST_FREE_BOND_AT;
        app.update();
        assert!(app.world().resource::<BondEconomy>().free_draw_available);
        app.world_mut().resource_mut::<GameSession>().elapsed = 300.0;
        app.update();
        assert!(app.world().resource::<BondEconomy>().free_draw_available);

        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 100);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 0);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Free)
        );

        press_action(&mut app, DraftAction::Later);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Free)
        );
        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 100);
        press_action(&mut app, DraftAction::Refresh);
        assert_eq!(app.world().resource::<GameSession>().wood, 100);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Free)
        );
        press_first_choice(&mut app);
        assert!(
            app.world()
                .resource::<BondEconomy>()
                .pending_payment
                .is_none()
        );

        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 90);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 1);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Wood(10))
        );
        press_action(&mut app, DraftAction::Abandon);
        assert!(
            app.world()
                .resource::<BondEconomy>()
                .pending_payment
                .is_none()
        );

        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 80);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 2);
        press_action(&mut app, DraftAction::Abandon);
        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 68);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 3);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Wood(12))
        );
    }

    #[test]
    fn full_skill_bar_only_rolls_owned_upgrades() {
        let mut build = PlayerBuild::default();
        for rank in &mut build.skills[..SKILL_SLOT_LIMIT] {
            *rank = 1;
        }
        let mut rng = RunRng::default();
        let selected = SelectedHero::default();
        let mut drought = DraftAntiDrought::default();
        for _ in 0..20 {
            let choices = pick_skill_choices(
                &mut rng,
                30,
                EPIC_SKILL_UNLOCK_AT,
                &build,
                &selected,
                &mut drought,
            );
            assert_eq!(choices.len(), 3);
            assert!(choices.into_iter().all(|choice| {
                matches!(choice, DraftChoice::Skill(skill) if (skill.0 as usize) < SKILL_SLOT_LIMIT)
            }));
        }
    }

    #[test]
    fn choose_later_reopens_without_a_second_charge() {
        let mut app = test_app();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = FIRST_FREE_SKILL_AT - 1.0;
            session.gold = 500;
        }

        tap_g(&mut app);
        assert_eq!(app.world().resource::<GameSession>().gold, 420);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 1);
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(80))
        );

        let later = {
            let world = app.world_mut();
            let mut actions = world.query::<(Entity, &DraftActionButton)>();
            actions
                .iter(world)
                .find_map(|(entity, action)| (action.0 == DraftAction::Later).then_some(entity))
                .expect("choose later button")
        };
        app.world_mut()
            .entity_mut(later)
            .insert(Interaction::Pressed);
        app.update();
        assert!(!app.world().resource::<GameSession>().paused);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_eq!(
            app.world().resource::<SkillEconomy>().pending_payment,
            Some(SkillDrawPayment::Gold(80))
        );

        tap_g(&mut app);
        assert_eq!(app.world().resource::<GameSession>().gold, 420);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 1);
        assert!(app.world().resource::<GameSession>().paused);

        let first_choice = {
            let world = app.world_mut();
            let mut choices = world.query::<(Entity, &DraftButton)>();
            choices
                .iter(world)
                .find_map(|(entity, choice)| (choice.0 == 0).then_some(entity))
                .expect("first skill choice")
        };
        app.world_mut()
            .entity_mut(first_choice)
            .insert(Interaction::Pressed);
        app.update();
        assert!(
            app.world()
                .resource::<SkillEconomy>()
                .pending_payment
                .is_none()
        );

        tap_g(&mut app);
        assert_eq!(app.world().resource::<GameSession>().gold, 325);
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
    }
}
