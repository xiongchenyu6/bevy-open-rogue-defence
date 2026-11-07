use bevy::prelude::*;

use super::content::{HEROES, Rarity, SKILLS, SkillEffectKind, Tag};
use super::encounters::EncounterState;
use super::market::MarketEffects;
use super::skill_evolution::SkillEvolution;
use super::{
    Autoplay, BondCardId, BondDrawButton, BondId, DraftAction, DraftActionButton, DraftButton,
    DraftChoice, DraftKind, DraftState, EncounterButton, GameLanguage, GameSession, HeroVitals,
    MAX_HERO_LEVEL, PlayerBuild, RecruitmentProgress, RunRng, SelectedHero, SkillDrawButton,
    SkillId,
};

#[path = "progression_ui.rs"]
mod presentation;
use presentation::{BuildPreview, spawn_draft_ui};

pub struct ProgressionPlugin;

impl Plugin for ProgressionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::selection_detail::SelectionDetail>()
            .init_resource::<Messages<bevy::input::mouse::MouseWheel>>()
            .init_resource::<Touches>()
            .add_systems(
                Update,
                (
                    super::selection_detail::toggle,
                    super::selection_detail::update_labels,
                )
                    .chain()
                    .before(presentation::update_feedback),
            )
            .init_resource::<SkillEvolution>()
            .add_systems(PreUpdate, super::skill_evolution::refresh_access);
        app.init_resource::<GameLanguage>()
            .init_resource::<SkillEconomy>()
            .init_resource::<RecruitmentProgress>()
            .init_resource::<BondEconomy>()
            .init_resource::<DraftAntiDrought>()
            .init_resource::<BuildPreview>()
            .init_resource::<DraftContinuation>()
            .add_systems(
                Update,
                (
                    check_level_up,
                    grant_free_skill_draws,
                    grant_first_free_bond_draw,
                    request_skill_draft,
                    request_bond_draft,
                    request_encounter_draft,
                    presentation::sync_preview,
                    handle_draft_buttons,
                    handle_draft_actions,
                    handle_draft_keys,
                    autoplay_draft,
                    presentation::load_images,
                    presentation::update_layout,
                    presentation::scroll_details,
                    presentation::update_feedback,
                    presentation::refresh_language,
                )
                    .chain(),
            );
    }
}

// The current full-run footage presents its guided G draft at ~00:04.
const FIRST_FREE_SKILL_AT: f32 = 4.0;
// The current full-run footage opens its first guided F draft at ~00:12.
const FIRST_FREE_BOND_AT: f32 = 12.0;
// Autoplay opens its first F draft at 0:18, then draws whenever it can pay.
const FIRST_AUTO_BOND_AT: f32 = 18.0;
const EPIC_SKILL_UNLOCK_AT: f32 = 12.0 * 60.0;
const FREE_SKILL_LEVELS: [u32; 5] = [5, 10, 15, 20, 25];
#[cfg(test)]
const SKILL_SLOT_LIMIT: usize = 6;
const BOND_SLOT_LIMIT: usize = 7;
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

/// Continues only the draw category the player explicitly entered.
#[derive(Resource, Default)]
struct DraftContinuation {
    queued: Option<DraftKind>,
    enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BondDrawPayment {
    Free,
    Wood(u32),
}

/// Owns the banked 0:12 grant and the in-flight F payment. `GameSession::bond_draws`
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
    let late_draws = economy.paid_draws.saturating_sub(2);
    80_u32
        .saturating_add(economy.paid_draws.saturating_mul(15))
        .saturating_add(late_draws.saturating_mul(late_draws).saturating_mul(20))
}

fn check_level_up(mut session: ResMut<GameSession>) {
    if session.finished
        || session.paused
        || session.save_active
        || session.level >= MAX_HERO_LEVEL
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
    language: Res<GameLanguage>,
    evolution: Res<SkillEvolution>,
    mut continuation: ResMut<DraftContinuation>,
) {
    if session.finished || session.paused || session.save_active || draft.root.is_some() {
        return;
    }
    let manual = keyboard.just_pressed(KeyCode::KeyG)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    let pressed = continuation.queued == Some(DraftKind::Skill)
        && (session.skill_points > 0 || session.gold >= skill_cost(&economy))
        || keyboard.just_pressed(KeyCode::KeyG)
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
        continuation.queued = None;
        if manual {
            continuation.enabled = true;
        }
        session.paused = true;
        spawn_draft_ui(
            &mut commands,
            &mut draft,
            DraftKind::Skill,
            session.skill_rerolls,
            *language,
        );
        return;
    }
    if draft.kind.is_some() {
        return;
    }
    continuation.queued = None;
    if manual || autoplay.is_some() {
        continuation.enabled = true;
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
        evolution.skill_capacity(),
    );
    let choices = late_skill_choices(choices, &build, &evolution, session.elapsed, &mut rng);
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
        *language,
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
    mut encounters: ResMut<EncounterState>,
    language: Res<GameLanguage>,
    mut continuation: ResMut<DraftContinuation>,
) {
    if session.finished || session.paused || session.save_active || draft.root.is_some() {
        return;
    }
    let can_pay = economy.pending_payment.is_some()
        || economy.free_draw_available
        || session.wood >= bond_cost(&session, &encounters);
    let automatic = autoplay.is_some()
        && session.auto_bond_draws
        && session.elapsed >= FIRST_AUTO_BOND_AT
        && can_pay;
    let manual = keyboard.just_pressed(KeyCode::KeyF)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    let pressed = continuation.queued == Some(DraftKind::Bond)
        || keyboard.just_pressed(KeyCode::KeyF)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
        || automatic;
    if !pressed {
        return;
    }
    continuation.queued = None;
    if manual || automatic {
        continuation.enabled = true;
    }
    if !can_pay {
        continuation.enabled = false;
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
        spawn_draft_ui(
            &mut commands,
            &mut draft,
            kind,
            session.skill_rerolls,
            *language,
        );
        return;
    }
    if draft.kind.is_some() {
        return;
    }
    let cost = bond_cost(&session, &encounters);
    if !economy.free_draw_available && session.wood < cost {
        return;
    }
    let compass_target = (build.bond_complete(BondId::Journey)
        && market_effects.bond_compass_available())
    .then(|| highest_progress_unfinished_bond(&build))
    .flatten();
    let choices = pick_bond_choices_at(
        &mut rng,
        &build,
        &selected,
        &mut drought,
        compass_target,
        &session,
    );
    if choices.is_empty() {
        continuation.enabled = false;
        return;
    }
    if compass_target.is_some_and(|target| {
        choices
            .iter()
            .any(|choice| matches!(choice, DraftChoice::Bond(card) if card.family() == target))
    }) {
        let _ = market_effects.consume_bond_compass();
    }
    economy.pending_payment = Some(if economy.free_draw_available {
        economy.free_draw_available = false;
        BondDrawPayment::Free
    } else {
        session.wood -= cost;
        session.bond_draws = session.bond_draws.saturating_add(1);
        encounters.consume_bond_levy();
        BondDrawPayment::Wood(cost)
    });
    open_draft(
        &mut commands,
        &mut session,
        &mut draft,
        DraftKind::Bond,
        choices,
        *language,
    );
}

/// Every paid draw costs 5 more wood with no cap. A full run's wood (~1,600 at
/// tower floor 14, ~2,400 at floor 20) buys roughly 22-28 draws, so a run
/// completes a handful of the 25 post-opening families instead of the pool.
/// A Timber Levy encounter raises the next three paid draws by half.
pub fn bond_cost(session: &GameSession, encounters: &EncounterState) -> u32 {
    encounters.bond_price(10 + 5 * session.bond_draws)
}

/// Encounters are free; the bank is spent when a hand is opened and the hand
/// stays open across "later" until an event is accepted. E or the HUD button
/// opens it; autoplay opens it as soon as one is banked.
fn request_encounter_draft(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    autoplay: Option<Res<Autoplay>>,
    interactions: Query<&Interaction, (With<EncounterButton>, Changed<Interaction>)>,
    mut session: ResMut<GameSession>,
    build: Res<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut encounters: ResMut<EncounterState>,
    mut rng: ResMut<RunRng>,
    language: Res<GameLanguage>,
    mut continuation: ResMut<DraftContinuation>,
) {
    if session.finished || session.paused || session.save_active || draft.root.is_some() {
        return;
    }
    let manual = keyboard.just_pressed(KeyCode::KeyE)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed);
    let pressed = continuation.queued == Some(DraftKind::Encounter)
        || keyboard.just_pressed(KeyCode::KeyE)
        || interactions
            .iter()
            .any(|interaction| *interaction == Interaction::Pressed)
        || (autoplay.is_some() && (encounters.banked > 0 || encounters.pending()));
    if !pressed {
        return;
    }
    continuation.queued = None;
    if manual || autoplay.is_some() {
        continuation.enabled = true;
    }
    if draft.kind == Some(DraftKind::Encounter) && !draft.choices.is_empty() {
        if !encounters.pending() {
            return;
        }
        session.paused = true;
        spawn_draft_ui(
            &mut commands,
            &mut draft,
            DraftKind::Encounter,
            session.skill_rerolls,
            *language,
        );
        return;
    }
    if draft.kind.is_some() {
        return;
    }
    let Some(hand) = encounters.open_hand(&mut rng, &build) else {
        continuation.enabled = false;
        return;
    };
    open_draft(
        &mut commands,
        &mut session,
        &mut draft,
        DraftKind::Encounter,
        hand.into_iter().map(DraftChoice::Encounter).collect(),
        *language,
    );
}

fn open_draft(
    commands: &mut Commands,
    session: &mut GameSession,
    draft: &mut DraftState,
    kind: DraftKind,
    choices: Vec<DraftChoice>,
    language: GameLanguage,
) {
    session.paused = true;
    draft.choices = choices;
    draft.kind = Some(kind);
    draft.pending_bond_replacement = None;
    draft.opened_for = 0.0;
    spawn_draft_ui(commands, draft, kind, session.skill_rerolls, language);
}

fn pick_skill_choices(
    rng: &mut RunRng,
    level: u32,
    elapsed: f32,
    build: &PlayerBuild,
    selected: &SelectedHero,
    drought: &mut DraftAntiDrought,
    capacity: usize,
) -> Vec<DraftChoice> {
    let slots_full = active_skill_slots(build) >= capacity;
    let pool: Vec<SkillId> = (0..SKILLS.len())
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
    let mut choices: Vec<SkillId> = Vec::new();
    let mut remaining = pool.clone();
    // Every slot is seeded and rarity-weighted; owned high ranks receive no guarantee.
    while choices.len() < 3 && !remaining.is_empty() {
        let has_ur = choices
            .iter()
            .any(|skill| skill.definition().rarity == Rarity::Ur);
        let Some(candidate) = take_weighted_skill(&mut remaining, rng, has_ur) else {
            break;
        };
        choices.push(candidate);
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
            choices[replacement] =
                choose_weighted_skill(&preferred, rng).expect("preferred pool is not empty");
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

fn late_skill_choices(
    mut choices: Vec<DraftChoice>,
    build: &PlayerBuild,
    evolution: &SkillEvolution,
    elapsed: f32,
    rng: &mut RunRng,
) -> Vec<DraftChoice> {
    let ready: Vec<_> = (0..SKILLS.len())
        .map(|index| SkillId(index as u8))
        .filter(|skill| evolution.branch_ready(*skill, build))
        .collect();
    if !ready.is_empty() {
        let skill = ready[rng.next_u32() as usize % ready.len()];
        return super::skill_evolution::UrEvolutionPath::ALL
            .into_iter()
            .map(|path| DraftChoice::EvolveUr { skill, path })
            .collect();
    }
    let upgrades: Vec<_> = (0..SKILLS.len())
        .map(|index| SkillId(index as u8))
        .filter(|skill| evolution.eligible(*skill, build))
        .collect();
    if !upgrades.is_empty() {
        choices.extend(upgrades.into_iter().map(|skill| DraftChoice::Evolution {
            skill,
            tier: evolution.tiers[usize::from(skill.0)] + 1,
        }));
        // Ascensions compete with ordinary upgrades, independent of skill-ID ordering.
        for index in (1..choices.len()).rev() {
            let other = rng.next_u32() as usize % (index + 1);
            choices.swap(index, other);
        }
        choices.truncate(3);
    }
    if active_skill_slots(build) >= evolution.skill_capacity()
        && elapsed >= EPIC_SKILL_UNLOCK_AT
        && !choices.iter().any(|choice| {
            matches!(choice,
            DraftChoice::Skill(skill) if skill.definition().rarity == Rarity::Ur)
        })
        && (choices.is_empty() || rng.next_u32() % 100 < Rarity::Ur.draw_weight())
    {
        let candidates: Vec<_> = (0..SKILLS.len())
            .map(|index| SkillId(index as u8))
            .filter(|skill| {
                skill.definition().rarity == Rarity::Ur && build.skill_rank(*skill) == 0
            })
            .collect();
        if !candidates.is_empty() {
            let new =
                candidates[(rng.unit() * candidates.len() as f32) as usize % candidates.len()];
            let old: Vec<_> = (0..SKILLS.len())
                .map(|index| SkillId(index as u8))
                .filter(|skill| build.skill_rank(*skill) > 0)
                .collect();
            let offset = (rng.unit() * old.len() as f32) as usize % old.len();
            let count = if choices.is_empty() { 3 } else { 1 };
            for index in 0..count {
                let replacement = DraftChoice::ReplaceSkill {
                    old: old[(offset + index) % old.len()],
                    new,
                };
                if choices.len() == 3 {
                    choices[2] = replacement;
                } else {
                    choices.push(replacement);
                }
            }
        }
    }
    choices
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

fn choose_weighted_skill(pool: &[SkillId], rng: &mut RunRng) -> Option<SkillId> {
    let total_weight: u32 = pool
        .iter()
        .map(|skill| skill.definition().rarity.draw_weight())
        .sum();
    if total_weight == 0 {
        return None;
    }
    choose_weighted_skill_for_roll(pool, rng.next_u32() % total_weight)
}

fn choose_weighted_skill_for_roll(pool: &[SkillId], mut roll: u32) -> Option<SkillId> {
    pool.iter().copied().find(|skill| {
        let weight = skill.definition().rarity.draw_weight();
        if roll < weight {
            true
        } else {
            roll -= weight;
            false
        }
    })
}

fn take_weighted_skill(pool: &mut Vec<SkillId>, rng: &mut RunRng, has_ur: bool) -> Option<SkillId> {
    let eligible: Vec<(usize, SkillId)> = pool
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, skill)| rarity_fits_skill_draw(has_ur, skill.definition().rarity))
        .collect();
    let choice = choose_weighted_skill(
        &eligible.iter().map(|(_, skill)| *skill).collect::<Vec<_>>(),
        rng,
    )?;
    let index = eligible
        .into_iter()
        .find_map(|(index, skill)| (skill == choice).then_some(index))?;
    Some(pool.swap_remove(index))
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

#[cfg(test)]
fn pick_bond_choices(
    rng: &mut RunRng,
    build: &PlayerBuild,
    selected: &SelectedHero,
    drought: &mut DraftAntiDrought,
    guaranteed: Option<BondId>,
) -> Vec<DraftChoice> {
    pick_bond_choices_from(
        rng,
        build,
        selected,
        drought,
        guaranteed,
        available_bond_cards(build),
    )
}
fn pick_bond_choices_at(
    rng: &mut RunRng,
    build: &PlayerBuild,
    selected: &SelectedHero,
    drought: &mut DraftAntiDrought,
    guaranteed: Option<BondId>,
    session: &GameSession,
) -> Vec<DraftChoice> {
    pick_bond_choices_from(
        rng,
        build,
        selected,
        drought,
        guaranteed,
        available_bond_cards_at(build, session),
    )
}
fn pick_bond_choices_from(
    rng: &mut RunRng,
    build: &PlayerBuild,
    selected: &SelectedHero,
    drought: &mut DraftAntiDrought,
    guaranteed: Option<BondId>,
    available: Vec<BondCardId>,
) -> Vec<DraftChoice> {
    if available.is_empty() {
        return Vec::new();
    }

    // Opening acquisition is always the three unique Journey cards. Refresh,
    // hero preference and compass guarantees cannot skip this introduction.
    if !build.bond_complete(BondId::Journey) {
        return available.into_iter().map(DraftChoice::Bond).collect();
    }

    let mut choices: Vec<BondCardId> = Vec::with_capacity(3);
    if let Some(guaranteed) = guaranteed.and_then(|family| {
        available
            .iter()
            .copied()
            .find(|card| card.family() == family)
    }) {
        choices.push(guaranteed);
    }
    if drought.bond_missed_preference {
        let preferred: Vec<BondCardId> = available
            .iter()
            .copied()
            .filter(|bond| !choices.contains(bond))
            .filter(|bond| bond_matches_preference(bond.family(), selected, build))
            .collect();
        if choices.len() < 3 && !preferred.is_empty() {
            choices.push(
                choose_weighted_bond(&preferred, rng, build)
                    .expect("preferred bond pool is not empty"),
            );
        }
    }

    while choices.len() < 3 && choices.len() < available.len() {
        let remaining: Vec<BondCardId> = available
            .iter()
            .copied()
            .filter(|bond| !choices.contains(bond))
            .collect();
        // The reference says agility-oriented draws gain weight. Treat that as
        // a x2 adaptation and weight the whole remaining pool after completion,
        // so the old preference buckets cannot cancel the multiplier.
        if [BondId::Agility, BondId::Strength, BondId::Intelligence]
            .into_iter()
            .any(|family| build.bond_complete(family))
        {
            choices.push(choose_weighted_bond(&remaining, rng, build).expect("remaining cards"));
            continue;
        }
        let existing: Vec<BondCardId> = remaining
            .iter()
            .copied()
            .filter(|bond| build.bond(bond.family()) > 0)
            .collect();
        let preferred: Vec<BondCardId> = remaining
            .iter()
            .copied()
            .filter(|bond| build.bond(bond.family()) == 0)
            .filter(|bond| bond_matches_preference(bond.family(), selected, build))
            .collect();
        let other: Vec<BondCardId> = remaining
            .iter()
            .copied()
            .filter(|bond| build.bond(bond.family()) == 0)
            .filter(|bond| !bond_matches_preference(bond.family(), selected, build))
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
        choices.push(
            choose_weighted_bond(candidates, rng, build)
                .expect("selected bond bucket is not empty"),
        );
    }

    drought.bond_missed_preference = !choices
        .iter()
        .any(|bond| bond_matches_preference(bond.family(), selected, build));
    choices.into_iter().map(DraftChoice::Bond).collect()
}

fn bond_card_draw_weight(card: BondCardId, build: &PlayerBuild) -> u32 {
    let boosted = match card.family() {
        BondId::Ranger | BondId::Archery => build.bond_complete(BondId::Agility),
        BondId::Warpath => build.bond_complete(BondId::Strength),
        BondId::Arcanum => build.bond_complete(BondId::Intelligence),
        _ => false,
    };
    let multiplier = if boosted { 2 } else { 1 };
    card.rarity().draw_weight() * multiplier
}

fn choose_weighted_bond(
    pool: &[BondCardId],
    rng: &mut RunRng,
    build: &PlayerBuild,
) -> Option<BondCardId> {
    let total_weight: u32 = pool
        .iter()
        .map(|card| bond_card_draw_weight(*card, build))
        .sum();
    if total_weight == 0 {
        return None;
    }
    choose_weighted_bond_for_roll(pool, rng.next_u32() % total_weight, build)
}

fn choose_weighted_bond_for_roll(
    pool: &[BondCardId],
    mut roll: u32,
    build: &PlayerBuild,
) -> Option<BondCardId> {
    pool.iter().copied().find(|bond| {
        let weight = bond_card_draw_weight(*bond, build);
        if roll < weight {
            true
        } else {
            roll -= weight;
            false
        }
    })
}

fn bond_card_available(card: BondCardId, build: &PlayerBuild) -> bool {
    !build.owns_bond_card(card)
        && (card.family() == BondId::Journey || build.bond_complete(BondId::Journey))
        && card
            .family()
            .definition()
            .prerequisite
            .is_none_or(|family| build.bond_complete(family))
}

pub fn available_bond_cards(build: &PlayerBuild) -> Vec<BondCardId> {
    BondCardId::ALL
        .into_iter()
        .filter(|card| bond_card_available(*card, build))
        .collect()
}

/// Alternate level/time gate, shared by drafting and the collection UI.
pub fn bond_stage_requirement(family: BondId) -> Option<(u32, f32)> {
    use BondId::*;
    match family {
        Cultivation | ElementMaster | Hunter | Barbarian | Windrunner | Requiem => {
            Some((20, 480.0))
        }
        SpiritEnergy | ElementArchmage | DemonHunter | Mountain | Sharpshooter => Some((40, 900.0)),
        MadScientist | ForbiddenArts | RevivalWind | KillingIntent | Destruction => {
            Some((50, 1200.0))
        }
        _ => None,
    }
}
pub fn bond_stage_available(card: BondCardId, session: &GameSession) -> bool {
    bond_stage_requirement(card.family())
        .is_none_or(|(level, seconds)| session.level >= level || session.elapsed >= seconds)
}
pub fn available_bond_cards_at(build: &PlayerBuild, session: &GameSession) -> Vec<BondCardId> {
    available_bond_cards(build)
        .into_iter()
        .filter(|card| bond_stage_available(*card, session))
        .collect()
}

fn highest_progress_unfinished_bond(build: &PlayerBuild) -> Option<BondId> {
    BondId::ALL
        .into_iter()
        .filter(|family| build.bond(*family) > 0 && !build.bond_complete(*family))
        .max_by_key(|family| (build.bond(*family), std::cmp::Reverse(family.index())))
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
    tags_match_preference(bond.definition().favored_tags, selected, build)
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

fn handle_draft_buttons(
    mut commands: Commands,
    interactions: Query<(&Interaction, &DraftButton), Changed<Interaction>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    touches: Res<Touches>,
    mut preview: ResMut<BuildPreview>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut skill_economy: ResMut<SkillEconomy>,
    mut bond_economy: ResMut<BondEconomy>,
    language: Res<GameLanguage>,
    mut recruitment: ResMut<RecruitmentProgress>,
    mut evolution: ResMut<SkillEvolution>,
    mut encounters: ResMut<EncounterState>,
    mut vitals: ResMut<HeroVitals>,
    mut rng: ResMut<RunRng>,
) {
    if draft.root.is_none() {
        return;
    }
    let held_pointer = mouse
        .as_deref()
        .is_some_and(|mouse| mouse.pressed(MouseButton::Left))
        || touches.iter().next().is_some();
    let fresh_pointer = mouse
        .as_deref()
        .is_some_and(|mouse| mouse.just_pressed(MouseButton::Left))
        || touches.iter_just_pressed().next().is_some();
    if held_pointer && !fresh_pointer {
        return;
    }
    let Some(index) = interactions.iter().find_map(|(interaction, button)| {
        (*interaction == Interaction::Pressed && button.0 < draft.choices.len()).then_some(button.0)
    }) else {
        return;
    };
    preview.index = Some(index);
    preview.root = draft.root;
    choose(
        index,
        &mut commands,
        &mut session,
        &mut build,
        &mut draft,
        &mut skill_economy,
        &mut bond_economy,
        *language,
        &mut recruitment,
        &mut evolution,
        &mut encounters,
        &mut vitals,
        &mut rng,
    );
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
    language: Res<GameLanguage>,
    evolution: Res<SkillEvolution>,
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
                if session.skill_rerolls == 0
                    || matches!(kind, DraftKind::BondReplacement | DraftKind::Encounter)
                {
                    return;
                }
                let choices = match kind {
                    DraftKind::Skill => pick_skill_choices(
                        &mut rng,
                        session.level,
                        session.elapsed,
                        &build,
                        &selected,
                        &mut drought,
                        evolution.skill_capacity(),
                    ),
                    DraftKind::Bond => pick_bond_choices_at(
                        &mut rng,
                        &build,
                        &selected,
                        &mut drought,
                        None,
                        &session,
                    ),
                    DraftKind::BondReplacement | DraftKind::Encounter => return,
                };
                let choices = if kind == DraftKind::Skill {
                    late_skill_choices(choices, &build, &evolution, session.elapsed, &mut rng)
                } else {
                    choices
                };
                if choices.is_empty() {
                    return;
                }
                session.skill_rerolls -= 1;
                draft.choices = choices;
                if let Some(root) = draft.root.take() {
                    commands.entity(root).despawn();
                }
                spawn_draft_ui(
                    &mut commands,
                    &mut draft,
                    kind,
                    session.skill_rerolls,
                    *language,
                );
            }
            DraftAction::Abandon => {
                if kind == DraftKind::Encounter {
                    return;
                }
                stop_draft_continuation(&mut commands);
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
                    stop_draft_continuation(&mut commands);
                    commands.entity(root).despawn();
                }
                draft.opened_for = 0.0;
                session.paused = false;
            }
        }
        break;
    }
}

fn stop_draft_continuation(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let mut continuation = world.resource_mut::<DraftContinuation>();
        continuation.queued = None;
        continuation.enabled = false;
    });
}

fn handle_draft_keys(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut preview: ResMut<BuildPreview>,
    mut session: ResMut<GameSession>,
    mut build: ResMut<PlayerBuild>,
    mut draft: ResMut<DraftState>,
    mut skill_economy: ResMut<SkillEconomy>,
    mut bond_economy: ResMut<BondEconomy>,
    language: Res<GameLanguage>,
    mut recruitment: ResMut<RecruitmentProgress>,
    mut evolution: ResMut<SkillEvolution>,
    mut encounters: ResMut<EncounterState>,
    mut vitals: ResMut<HeroVitals>,
    mut rng: ResMut<RunRng>,
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
    if let Some(index) = choice.filter(|index| *index < draft.choices.len())
        && preview.root == draft.root
    {
        preview.index = Some(index);
        choose(
            index,
            &mut commands,
            &mut session,
            &mut build,
            &mut draft,
            &mut skill_economy,
            &mut bond_economy,
            *language,
            &mut recruitment,
            &mut evolution,
            &mut encounters,
            &mut vitals,
            &mut rng,
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
    language: Res<GameLanguage>,
    mut recruitment: ResMut<RecruitmentProgress>,
    mut evolution: ResMut<SkillEvolution>,
    mut encounters: ResMut<EncounterState>,
    mut vitals: ResMut<HeroVitals>,
    mut rng: ResMut<RunRng>,
) {
    if autoplay.is_none() || draft.root.is_none() {
        return;
    }
    draft.opened_for += time.delta_secs();
    if draft.opened_for >= 1.15 {
        let index = if draft.kind == Some(DraftKind::Encounter) {
            // The balance bot takes the first event it can afford.
            draft
                .choices
                .iter()
                .position(|choice| {
                    matches!(choice, DraftChoice::Encounter(event) if event.affordable(&session))
                })
                .unwrap_or(0)
        } else if draft.kind == Some(DraftKind::BondReplacement) {
            draft
                .choices
                .iter()
                .enumerate()
                .filter_map(|(index, choice)| match choice {
                    DraftChoice::ReplaceBond(bond) => Some((index, *bond)),
                    _ => None,
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
            *language,
            &mut recruitment,
            &mut evolution,
            &mut encounters,
            &mut vitals,
            &mut rng,
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
    language: GameLanguage,
    recruitment: &mut RecruitmentProgress,
    evolution: &mut SkillEvolution,
    encounters: &mut EncounterState,
    vitals: &mut HeroVitals,
    rng: &mut RunRng,
) {
    let Some(choice) = draft.choices.get(index).copied() else {
        return;
    };
    match choice {
        DraftChoice::Skill(skill) => {
            if skill_economy.pending_payment.is_none() {
                return;
            }
            // New skills at a full bar require an explicit old → new replacement card.
            if build.skill_rank(skill) == 0
                && active_skill_slots(build) >= evolution.skill_capacity()
            {
                return;
            }
            grant_skill_rank(skill, session, build, evolution);
        }
        DraftChoice::Evolution { skill, tier } => {
            let index = usize::from(skill.0);
            if skill_economy.pending_payment.is_none()
                || !evolution.eligible(skill, build)
                || tier != evolution.tiers[index] + 1
            {
                return;
            }
            evolution.tiers[index] = tier;
        }
        DraftChoice::EvolveUr { skill, path } => {
            if skill_economy.pending_payment.is_none() || !evolution.branch_ready(skill, build) {
                return;
            }
            evolution.paths[usize::from(skill.0)] = Some(path);
            evolution.tiers[usize::from(skill.0)] = 0;
        }
        DraftChoice::ReplaceSkill { old, new } => {
            if skill_economy.pending_payment.is_none()
                || build.skill_rank(old) == 0
                || build.skill_rank(new) > 0
                || new.definition().rarity != Rarity::Ur
                || session.elapsed < EPIC_SKILL_UNLOCK_AT
            {
                return;
            }
            evolution.rewarded_ranks[usize::from(old.0)] =
                evolution.rewarded_ranks[usize::from(old.0)].max(build.skill_rank(old));
            build.skills[usize::from(old.0)] = 0;
            evolution.tiers[usize::from(old.0)] = 0;
            evolution.paths[usize::from(old.0)] = None;
            build.skills[usize::from(new.0)] = 1;
            evolution.rewarded_ranks[usize::from(new.0)] =
                evolution.rewarded_ranks[usize::from(new.0)].max(1);
        }
        DraftChoice::Bond(card) => {
            if bond_economy.pending_payment.is_none()
                || draft.kind != Some(DraftKind::Bond)
                || !bond_card_available(card, build)
                || !bond_stage_available(card, session)
            {
                return;
            }
            if build.bond(card.family()) == 0 && build.active_bond_count() >= BOND_SLOT_LIMIT {
                begin_bond_replacement(
                    card,
                    commands,
                    session,
                    build,
                    draft,
                    session.skill_rerolls,
                    language,
                );
                return;
            }
            recruitment.fruit_count = recruitment
                .fruit_count
                .saturating_add(apply_bond(card, session, build));
        }
        DraftChoice::ReplaceBond(family) => {
            let Some(card) = draft.pending_bond_replacement else {
                return;
            };
            if bond_economy.pending_payment.is_none()
                || draft.kind != Some(DraftKind::BondReplacement)
                || build.bond(family) == 0
                || build.bond_complete(family)
                || !bond_card_available(card, build)
                || !bond_stage_available(card, session)
                || build.bond(card.family()) > 0
            {
                return;
            }
            recruitment.fruit_count = recruitment
                .fruit_count
                .saturating_add(replace_bond(family, card, session, build));
        }
        DraftChoice::Encounter(event) => {
            if draft.kind != Some(DraftKind::Encounter)
                || !encounters.accept(event, session, build, vitals, rng)
            {
                return;
            }
        }
    }
    let next = match draft.kind {
        Some(DraftKind::Skill)
            if session.skill_points > 0 || session.gold >= skill_cost(skill_economy) =>
        {
            Some(DraftKind::Skill)
        }
        Some(DraftKind::Bond | DraftKind::BondReplacement)
            if bond_economy.free_draw_available
                || session.wood >= bond_cost(session, encounters) =>
        {
            Some(DraftKind::Bond)
        }
        Some(DraftKind::Encounter) if encounters.banked > 0 => Some(DraftKind::Encounter),
        _ => None,
    };
    clear_draft(commands, session, draft, skill_economy, bond_economy);
    commands.queue(move |world: &mut World| {
        let mut continuation = world.resource_mut::<DraftContinuation>();
        continuation.queued = if continuation.enabled { next } else { None };
    });
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
    new_bond: BondCardId,
    commands: &mut Commands,
    session: &mut GameSession,
    build: &PlayerBuild,
    draft: &mut DraftState,
    rerolls: u32,
    language: GameLanguage,
) {
    let choices: Vec<DraftChoice> = BondId::ALL
        .into_iter()
        .filter(|bond| build.bond(*bond) > 0 && !build.bond_complete(*bond))
        .take(BOND_SLOT_LIMIT)
        .map(DraftChoice::ReplaceBond)
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
    spawn_draft_ui(
        commands,
        draft,
        DraftKind::BondReplacement,
        rerolls,
        language,
    );
}

fn replace_bond(
    old_bond: BondId,
    new_bond: BondCardId,
    session: &mut GameSession,
    build: &mut PlayerBuild,
) -> u32 {
    remove_bond(old_bond, build);
    apply_bond(new_bond, session, build)
}

pub(crate) fn remove_bond(bond: BondId, build: &mut PlayerBuild) {
    for card in bond.definition().members {
        build.owned_bond_cards[card.index()] = false;
    }
}

fn grant_skill_rank(
    skill: SkillId,
    session: &mut GameSession,
    build: &mut PlayerBuild,
    evolution: &mut SkillEvolution,
) {
    let index = usize::from(skill.0);
    let next = (build.skills[index] + 1).min(skill.definition().max_rank);
    if next == build.skills[index] {
        return;
    }
    if next > evolution.rewarded_ranks[index] {
        apply_skill(skill, session, build);
        evolution.rewarded_ranks[index] = next;
    } else {
        build.skills[index] = next;
    }
}

fn apply_skill(skill: SkillId, session: &mut GameSession, build: &mut PlayerBuild) {
    let def = skill.definition();
    let rarity_power = def.rarity.skill_power_multiplier();
    let rank = (build.skills[skill.0 as usize] + 1).min(def.max_rank);
    build.skills[skill.0 as usize] = rank;

    match def.effect.kind {
        SkillEffectKind::Projectile | SkillEffectKind::AttackProc => {
            build.attack_damage += 2.5 * rarity_power;
            build.projectile_speed *= 1.0 + 0.025 * rarity_power;
        }
        SkillEffectKind::Nova | SkillEffectKind::GroundZone => {
            build.attack_damage += 3.5 * rarity_power;
        }
        SkillEffectKind::Orbit | SkillEffectKind::Echo => {
            build.attacks_per_second *= 1.0 + 0.025 * rarity_power;
        }
        SkillEffectKind::Summon => {
            if rank == 1 || rank == 4 {
                build.multishot = build.multishot.saturating_add(1).min(7);
            }
        }
        SkillEffectKind::Mark => build.pierce = build.pierce.saturating_add(1).min(12),
        SkillEffectKind::Revive => {
            session.core_max_hp += 80.0 * rarity_power;
            session.heal_gate(180.0 * rarity_power);
        }
    }
    if def.tags.contains(&Tag::Storm) {
        build.attacks_per_second *= 1.0 + 0.018 * rarity_power;
    }
    if def.tags.contains(&Tag::Flame) {
        build.attack_damage += 2.0 * rarity_power;
    }
    if def.tags.contains(&Tag::Guard) {
        session.heal_gate(35.0 * rarity_power);
    }
}

/// Settle acquisition rewards once. The caller deposits returned Job Fruits into recruitment.
fn apply_bond(card: BondCardId, session: &mut GameSession, build: &mut PlayerBuild) -> u32 {
    use super::bond_catalog::BondEffect;
    if !bond_card_available(card, build) {
        return 0;
    }
    let family = card.family();
    let previous_count = build.bond(family);
    build.owned_bond_cards[card.index()] = true;
    let new_count = build.bond(family);
    let mut fruits = 0_u32;
    let mut settle = |effect: &BondEffect| match effect {
        BondEffect::Wood(amount) => session.wood = session.wood.saturating_add(*amount),
        BondEffect::Gold(amount) => session.gold = session.gold.saturating_add(*amount),
        BondEffect::JobFruit(amount) => fruits = fruits.saturating_add(*amount),
        _ => {}
    };
    for effect in card.definition().effects {
        settle(effect);
    }
    for bonus in family.definition().bonuses {
        if previous_count < bonus.required && new_count >= bonus.required {
            for effect in bonus.effects {
                settle(effect);
            }
        }
    }
    fruits
}

pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(SkillEconomy::default());
    world.insert_resource(BondEconomy::default());
    world.insert_resource(DraftAntiDrought::default());
    world.insert_resource(BuildPreview::default());
    world.insert_resource(DraftContinuation::default());
}

#[cfg(test)]
#[path = "progression_draft_tests.rs"]
mod draft_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::DraftRoot;

    pub(super) fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<DraftState>()
            .init_resource::<RunRng>()
            .init_resource::<SelectedHero>()
            .init_resource::<MarketEffects>()
            .init_resource::<HeroVitals>()
            .init_resource::<EncounterState>()
            .add_plugins(ProgressionPlugin);
        app.update();
        app
    }

    #[test]
    fn all_six_mastered_skills_have_three_late_stages_and_relearning_cannot_farm_stats() {
        let mut build = PlayerBuild::default();
        build.skills.fill(0);
        build.skills[..6].fill(5);
        let mut evolution = SkillEvolution {
            unlocked_stage: 3,
            ..default()
        };
        let mut rng = RunRng::default();
        // Stages may now be offered in any order; every eligible skill remains reachable.
        for _ in 0..18 {
            let choices = late_skill_choices(Vec::new(), &build, &evolution, 0.0, &mut rng);
            if choices.is_empty() {
                break;
            }
            for choice in choices {
                let DraftChoice::Evolution {
                    skill,
                    tier: offered,
                } = choice
                else {
                    panic!("expected mastery offer")
                };
                assert_eq!(offered, evolution.tiers[usize::from(skill.0)] + 1);
                assert!(offered <= 3);
                evolution.tiers[usize::from(skill.0)] = offered;
            }
        }
        assert_eq!(&evolution.tiers[..6], &[3; 6]);
        assert!(late_skill_choices(Vec::new(), &build, &evolution, 0.0, &mut rng).is_empty());
        let skill = SkillId(1);
        let mut session = GameSession::default();
        build.skills[1] = 0;
        evolution.rewarded_ranks[1] = 0;
        for _ in 0..5 {
            grant_skill_rank(skill, &mut session, &mut build, &mut evolution);
        }
        let stats = (
            build.attack_damage,
            build.projectile_speed,
            build.attacks_per_second,
        );
        build.skills[1] = 0;
        for _ in 0..5 {
            grant_skill_rank(skill, &mut session, &mut build, &mut evolution);
        }
        assert_eq!(
            (
                build.attack_damage,
                build.projectile_speed,
                build.attacks_per_second
            ),
            stats
        );
    }

    #[test]
    fn each_mastered_ur_offers_three_exclusive_routes_and_real_card_click_preserves_rank() {
        use super::super::skill_evolution::UrEvolutionPath;
        for index in 26..=29 {
            for wanted in UrEvolutionPath::ALL {
                let mut app = test_app();
                app.world_mut().resource_mut::<PlayerBuild>().skills.fill(0);
                app.world_mut().resource_mut::<PlayerBuild>().skills[index] = 5;
                app.world_mut().resource_mut::<GameSession>().gold = 5000;
                tap_g(&mut app);
                let choices = app.world().resource::<DraftState>().choices.clone();
                assert_eq!(choices.len(), 3);
                for (choice, path) in choices.iter().zip(UrEvolutionPath::ALL) {
                    assert_eq!(
                        *choice,
                        DraftChoice::EvolveUr {
                            skill: SkillId(index as u8),
                            path
                        }
                    );
                }
                press_choice(&mut app, wanted.index());
                assert_eq!(
                    app.world().resource::<SkillEvolution>().paths[index],
                    Some(wanted)
                );
                assert_eq!(app.world().resource::<PlayerBuild>().skills[index], 5);
                assert_eq!(app.world().resource::<SkillEvolution>().tiers[index], 0);
                assert!(
                    !app.world()
                        .resource::<SkillEvolution>()
                        .branch_ready(SkillId(index as u8), app.world().resource::<PlayerBuild>())
                );
                assert!(
                    !app.world()
                        .resource::<SkillEvolution>()
                        .eligible(SkillId(index as u8), app.world().resource::<PlayerBuild>())
                );
            }
        }
    }

    #[test]
    fn completed_weapon_families_grant_one_fruit_only_on_final_unique_card() {
        for family in [BondId::Warpath, BondId::Arcanum, BondId::Archery] {
            let mut session = GameSession::default();
            let mut build = PlayerBuild::default();
            build.owned_bond_cards.fill(true);
            for card in family.definition().members {
                build.owned_bond_cards[card.index()] = false;
            }
            let members = family.definition().members;
            for (index, card) in members.iter().enumerate() {
                assert_eq!(
                    apply_bond(*card, &mut session, &mut build),
                    u32::from(index + 1 == members.len())
                );
                assert_eq!(
                    apply_bond(*card, &mut session, &mut build),
                    0,
                    "duplicate completion must not award again"
                );
            }
        }
    }
    #[test]
    fn fruit_requires_paid_card_acquisition_and_is_deposited_once() {
        let mut app = test_app();
        let card = BondCardId::WarpathFury;
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.owned_bond_cards.fill(true);
            build.owned_bond_cards[card.index()] = false;
        }
        install_test_bond_draft(
            &mut app,
            vec![DraftChoice::Bond(card)],
            None,
            BondDrawPayment::Free,
        );
        app.world_mut()
            .resource_mut::<BondEconomy>()
            .pending_payment = None;
        let before = (
            app.world().resource::<GameSession>().gold,
            app.world().resource::<GameSession>().wood,
        );
        press_first_choice(&mut app);
        assert!(!app.world().resource::<PlayerBuild>().owns_bond_card(card));
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 0);
        assert_eq!(
            (
                app.world().resource::<GameSession>().gold,
                app.world().resource::<GameSession>().wood
            ),
            before
        );
        app.world_mut()
            .resource_mut::<BondEconomy>()
            .pending_payment = Some(BondDrawPayment::Free);
        press_first_choice(&mut app);
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 1);
        install_test_bond_draft(
            &mut app,
            vec![DraftChoice::Bond(card)],
            None,
            BondDrawPayment::Free,
        );
        press_first_choice(&mut app);
        assert_eq!(app.world().resource::<RecruitmentProgress>().fruit_count, 1);
    }
    #[test]
    fn level_progression_reaches_forty_and_stops_at_sixty() {
        let mut app = App::new();
        app.insert_resource(GameSession {
            xp: 1_000_000,
            paused: false,
            ..Default::default()
        })
        .add_systems(Update, check_level_up);
        for _ in 0..39 {
            app.update();
        }
        assert_eq!(app.world().resource::<GameSession>().level, 40);
        for _ in 0..30 {
            app.update();
        }
        assert_eq!(app.world().resource::<GameSession>().level, MAX_HERO_LEVEL);
        let xp = app.world().resource::<GameSession>().xp;
        app.update();
        assert_eq!(app.world().resource::<GameSession>().xp, xp);
    }
    #[test]
    fn three_consecutive_card_clicks_and_number_key_acquire_without_confirmation() {
        let mut app = test_app();
        app.world_mut().resource_mut::<GameSession>().skill_points = 4;
        tap_g(&mut app);
        for index in [0, 2, 1] {
            let choice = app.world().resource::<DraftState>().choices[index];
            let DraftChoice::Skill(skill) = choice else {
                panic!("expected skill");
            };
            let before = app.world().resource::<PlayerBuild>().skill_rank(skill);
            press_choice(&mut app, index);
            assert_eq!(
                app.world().resource::<PlayerBuild>().skill_rank(skill),
                before + 1
            );
            assert!(app.world().resource::<DraftState>().root.is_none());
            assert!(!app.world().resource::<GameSession>().paused);
            app.update();
            assert_eq!(
                app.world().resource::<PlayerBuild>().skill_rank(skill),
                before + 1
            );
        }
        let DraftChoice::Skill(skill) = app.world().resource::<DraftState>().choices[1] else {
            panic!("expected skill");
        };
        let before = app.world().resource::<PlayerBuild>().skill_rank(skill);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Digit2);
        app.update();
        assert_eq!(
            app.world().resource::<PlayerBuild>().skill_rank(skill),
            before + 1
        );
        assert!(app.world().resource::<DraftState>().root.is_none());
    }
    #[test]
    fn held_pointer_cannot_consume_a_new_draft_until_another_press() {
        let mut app = test_app();
        app.init_resource::<ButtonInput<MouseButton>>();
        app.world_mut().resource_mut::<GameSession>().skill_points = 2;
        tap_g(&mut app);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        press_choice(&mut app, 0);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
        tap_g(&mut app);
        let before = app.world().resource::<PlayerBuild>().skills;
        press_choice(&mut app, 1);
        assert_eq!(app.world().resource::<PlayerBuild>().skills, before);
        assert!(app.world().resource::<DraftState>().root.is_some());
        app.update();
        assert_eq!(app.world().resource::<PlayerBuild>().skills, before);
        let button = {
            let world = app.world_mut();
            let mut cards = world.query::<(Entity, &DraftButton)>();
            cards
                .iter(world)
                .find(|(_, button)| button.0 == 1)
                .unwrap()
                .0
        };
        app.world_mut().entity_mut(button).insert(Interaction::None);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        press_choice(&mut app, 1);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_ne!(app.world().resource::<PlayerBuild>().skills, before);
    }

    #[test]
    fn rarity_weight_boundaries_drive_skill_and_card_candidates() {
        let skills = [SkillId(0), SkillId(12), SkillId(20), SkillId(26)];
        for (roll, index) in [
            (0, 0),
            (51, 0),
            (52, 1),
            (79, 1),
            (80, 2),
            (94, 2),
            (95, 3),
            (99, 3),
        ] {
            assert_eq!(
                choose_weighted_skill_for_roll(&skills, roll),
                Some(skills[index])
            );
        }
        let cards = [
            BondCardId::JourneyStrength,
            BondCardId::JudgeBrush,
            BondCardId::BookOfLife,
        ];
        for (roll, index) in [(0, 0), (51, 0), (52, 1), (79, 1), (80, 2), (94, 2)] {
            assert_eq!(
                choose_weighted_bond_for_roll(&cards, roll, &PlayerBuild::default()),
                Some(cards[index])
            );
        }
    }

    #[test]
    fn agility_completion_doubles_ranger_weight_without_boosting_other_families() {
        let cards = [BondCardId::RangerRapidFire, BondCardId::DivineMight];
        let mut build = PlayerBuild::default();
        assert_eq!(
            bond_card_draw_weight(cards[0], &build),
            bond_card_draw_weight(cards[1], &build)
        );
        for card in BondId::Journey
            .definition()
            .members
            .iter()
            .chain(BondId::Agility.definition().members)
        {
            build.owned_bond_cards[card.index()] = true;
        }
        let mut counts = [0; 2];
        let total: u32 = cards
            .iter()
            .map(|card| bond_card_draw_weight(*card, &build))
            .sum();
        for roll in 0..total {
            let chosen = choose_weighted_bond_for_roll(&cards, roll, &build).unwrap();
            counts[cards.iter().position(|card| *card == chosen).unwrap()] += 1;
        }
        assert_eq!(counts, [104, 52]);
        let mut rng = RunRng(0x12345678);
        for _ in 0..50 {
            let choices = pick_bond_choices(
                &mut rng,
                &build,
                &SelectedHero::default(),
                &mut DraftAntiDrought::default(),
                None,
            );
            for (index, choice) in choices.iter().enumerate() {
                let DraftChoice::Bond(card) = *choice else {
                    panic!("expected card");
                };
                assert!(bond_card_available(card, &build));
                assert!(!choices[..index].contains(choice));
            }
        }
    }

    #[test]
    fn distinct_cards_complete_family_and_reward_exactly_once() {
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let (gold, wood, attack) = (session.gold, session.wood, build.attack_damage);
        for card in BondId::Journey.definition().members {
            apply_bond(*card, &mut session, &mut build);
            apply_bond(*card, &mut session, &mut build);
        }
        assert_eq!(build.bond(BondId::Journey), 3);
        assert!(build.bond_complete(BondId::Journey));
        assert_eq!(build.active_bond_count(), 0);
        assert_eq!((session.gold, session.wood), (gold + 100, wood + 160));
        assert_eq!(build.attack_damage, attack);
        apply_bond(BondCardId::BookOfLife, &mut session, &mut build);
        assert_eq!(build.bond(BondId::Underworld), 1);
        assert!(!build.bond_complete(BondId::Underworld));
    }

    pub(super) fn tap_g(app: &mut App) {
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

    pub(super) fn press_choice(app: &mut App, wanted: usize) {
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
        choices: Vec<DraftChoice>,
        replacement: Option<BondCardId>,
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
            draft.choices = choices;
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
    fn skill_price_preserves_opening_costs_and_keeps_increasing() {
        let mut economy = SkillEconomy::default();
        assert_eq!(skill_cost(&economy), 80);
        economy.paid_draws = 1;
        assert_eq!(skill_cost(&economy), 95);
        economy.paid_draws = 2;
        assert_eq!(skill_cost(&economy), 110);
        economy.paid_draws = 12;
        assert_eq!(skill_cost(&economy), 2260);
        economy.paid_draws = 30;
        assert_eq!(skill_cost(&economy), 16210);
        economy.paid_draws = u32::MAX;
        assert_eq!(skill_cost(&economy), u32::MAX);
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
            SKILL_SLOT_LIMIT,
        );
        assert!(skills.iter().any(|choice| {
            matches!(choice, DraftChoice::Skill(skill)
                if tags_match_preference(skill.definition().tags, &selected, &build))
        }));
        assert!(!drought.skill_missed_preference);

        // Preference protection applies after the fixed Journey introduction.
        for card in BondId::Journey.definition().members {
            build.owned_bond_cards[card.index()] = true;
        }
        // Journey was the only Fortune family; use a mage with an unlocked
        // Intelligence preference to exercise post-introduction protection.
        let selected = SelectedHero {
            kind: super::super::HeroKind::StormSavant,
            locked: true,
        };
        assert!(
            available_bond_cards(&build)
                .iter()
                .any(|card| bond_matches_preference(card.family(), &selected, &build))
        );
        let bonds = pick_bond_choices(&mut rng, &build, &selected, &mut drought, None);
        assert!(bonds.iter().any(|choice| {
            matches!(choice, DraftChoice::Bond(bond)
                if bond_matches_preference(bond.family(), &selected, &build))
        }));
        assert!(!drought.bond_missed_preference);
    }

    #[test]
    fn opening_three_acquisitions_are_journey_for_every_hero_and_seed() {
        for hero in super::super::HeroKind::ALL {
            for seed in [1, 0xA11CE, u64::MAX] {
                let mut rng = RunRng(seed);
                let mut build = PlayerBuild::default();
                let mut session = GameSession::default();
                let mut drought = DraftAntiDrought {
                    bond_missed_preference: true,
                    ..default()
                };
                let selected = SelectedHero {
                    kind: hero,
                    locked: true,
                };
                for remaining in (1..=3).rev() {
                    let expected: Vec<_> = BondId::Journey
                        .definition()
                        .members
                        .iter()
                        .copied()
                        .filter(|card| !build.owns_bond_card(*card))
                        .collect();
                    assert_eq!(available_bond_cards(&build), expected);
                    // Refresh repeatedly with an unrelated compass guarantee.
                    for _ in 0..4 {
                        let hand = pick_bond_choices(
                            &mut rng,
                            &build,
                            &selected,
                            &mut drought,
                            Some(BondId::Ascension),
                        );
                        assert_eq!(hand.len(), remaining);
                        assert!(hand.iter().zip(&expected).all(|(choice, expected)|
                            matches!(choice, DraftChoice::Bond(card) if card == expected)));
                    }
                    apply_bond(*expected.last().unwrap(), &mut session, &mut build);
                }
                assert!(build.bond_complete(BondId::Journey));
                let pool = available_bond_cards(&build);
                for card in [
                    BondCardId::StrengthTraining,
                    BondCardId::AgilityTraining,
                    BondCardId::IntelligenceTraining,
                ] {
                    assert!(pool.contains(&card));
                }
                assert!(pool.iter().all(|card| card.family() != BondId::Journey));
            }
        }
    }

    #[test]
    fn card_pool_unlocks_in_order_and_excludes_owned_cards() {
        let mut build = PlayerBuild::default();
        let mut session = GameSession::default();
        assert!(
            available_bond_cards(&build)
                .iter()
                .all(|c| !matches!(c.family(), BondId::Agility | BondId::Ranger))
        );
        for card in BondId::Journey.definition().members {
            apply_bond(*card, &mut session, &mut build);
        }
        assert!(available_bond_cards(&build).contains(&BondCardId::AgilityTraining));
        assert!(!available_bond_cards(&build).contains(&BondCardId::RangerDeadeye));
        for card in BondId::Agility.definition().members {
            apply_bond(*card, &mut session, &mut build);
        }
        assert!(available_bond_cards(&build).contains(&BondCardId::RangerDeadeye));
        assert!(
            available_bond_cards(&build)
                .iter()
                .all(|c| !build.owns_bond_card(*c))
        );
        let mut rng = RunRng::default();
        for _ in 0..50 {
            let choices = pick_bond_choices(
                &mut rng,
                &build,
                &SelectedHero::default(),
                &mut DraftAntiDrought::default(),
                None,
            );
            for (index, choice) in choices.iter().enumerate() {
                assert!(!choices[..index].contains(choice));
            }
        }
    }

    #[test]
    fn real_ecs_draw_refresh_later_and_card_click_traverse_the_entire_card_chain() {
        let mut app = test_app();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 30.;
            session.level = 50;
            session.wood = 100_000;
            session.skill_rerolls = 1000;
        }
        for acquired in 0..BondCardId::COUNT {
            tap_f(&mut app);
            assert!(app.world().resource::<DraftState>().root.is_some());
            let choices = app.world().resource::<DraftState>().choices.clone();
            let wood = app.world().resource::<GameSession>().wood;
            let rng = app.world().resource::<RunRng>().0;
            press_action(&mut app, DraftAction::Later);
            tap_f(&mut app);
            assert_eq!(app.world().resource::<DraftState>().choices, choices);
            assert_eq!(app.world().resource::<GameSession>().wood, wood);
            assert_eq!(app.world().resource::<RunRng>().0, rng);
            press_action(&mut app, DraftAction::Refresh);
            let choices = app.world().resource::<DraftState>().choices.clone();
            let build = app.world().resource::<PlayerBuild>();
            if acquired < 3 {
                assert_eq!(choices.len(), 3 - acquired);
                assert!(choices.iter().all(|choice| matches!(choice,
                    DraftChoice::Bond(card) if card.family() == BondId::Journey)));
            }
            for (index, choice) in choices.iter().enumerate() {
                let DraftChoice::Bond(card) = *choice else {
                    panic!("expected card");
                };
                assert!(bond_card_available(card, build));
                assert!(bond_stage_available(
                    card,
                    app.world().resource::<GameSession>()
                ));
                assert!(!choices[..index].contains(choice));
            }
            // Finish an in-progress family where possible. At seven occupied
            // slots, use the real refresh control instead of discarding cards;
            // this traversal must acquire every unique card exactly once.
            let chosen = (0..100)
                .find_map(|_| {
                    let build = app.world().resource::<PlayerBuild>();
                    let choices = &app.world().resource::<DraftState>().choices;
                    let index = choices
                        .iter()
                        .enumerate()
                        .filter_map(|(index, choice)| {
                            let DraftChoice::Bond(card) = choice else {
                                return None;
                            };
                            let owned = build.bond(card.family());
                            (owned > 0 || build.active_bond_count() < BOND_SLOT_LIMIT)
                                .then_some((index, owned))
                        })
                        .max_by_key(|(_, owned)| *owned)
                        .map(|(index, _)| index);
                    if index.is_none() {
                        press_action(&mut app, DraftAction::Refresh);
                    }
                    index
                })
                .expect("refresh should offer an unfinished family within bounded attempts");
            press_choice(&mut app, chosen);
            assert_eq!(
                app.world()
                    .resource::<PlayerBuild>()
                    .owned_bond_cards
                    .iter()
                    .filter(|owned| **owned)
                    .count(),
                acquired + 1
            );
        }
        let build = app.world().resource::<PlayerBuild>();
        assert!(
            BondId::ALL
                .into_iter()
                .all(|family| build.bond_complete(family))
        );
        assert_eq!(build.active_bond_count(), 0);
        let wood = app.world().resource::<GameSession>().wood;
        tap_f(&mut app);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_eq!(app.world().resource::<GameSession>().wood, wood);
    }

    #[test]
    fn each_attribute_route_unlocks_its_advanced_family_through_ecs_card_click() {
        for (parent, advanced) in [
            (BondId::Strength, BondId::Warpath),
            (BondId::Intelligence, BondId::Arcanum),
            (BondId::Ranger, BondId::Archery),
        ] {
            let mut app = test_app();
            {
                let mut build = app.world_mut().resource_mut::<PlayerBuild>();
                build.owned_bond_cards.fill(true);
                for card in parent
                    .definition()
                    .members
                    .iter()
                    .chain(advanced.definition().members)
                {
                    build.owned_bond_cards[card.index()] = false;
                }
            }
            {
                let mut session = app.world_mut().resource_mut::<GameSession>();
                session.elapsed = 30.;
                session.wood = 1000;
            }
            for owned in 0..parent.required() {
                tap_f(&mut app);
                assert!(app.world().resource::<DraftState>().choices.iter().all(
                    |choice| matches!(choice, DraftChoice::Bond(card) if card.family() == parent)
                ));
                press_first_choice(&mut app);
                let build = app.world().resource::<PlayerBuild>();
                assert_eq!(build.bond(parent), owned + 1);
                assert_eq!(build.bond_complete(parent), owned + 1 == parent.required());
            }
            tap_f(&mut app);
            assert!(app.world().resource::<DraftState>().choices.iter().all(
                |choice| matches!(choice, DraftChoice::Bond(card) if card.family() == advanced)
            ));
            press_first_choice(&mut app);
            assert_eq!(app.world().resource::<PlayerBuild>().bond(advanced), 1);
        }
    }

    #[test]
    fn clicking_injected_locked_card_cannot_acquire_or_reward_it() {
        let mut app = test_app();
        install_test_bond_draft(
            &mut app,
            vec![DraftChoice::Bond(BondCardId::AgilityTraining)],
            None,
            BondDrawPayment::Free,
        );
        let before = app.world().resource::<PlayerBuild>().owned_bond_cards;
        press_first_choice(&mut app);
        assert_eq!(
            app.world().resource::<PlayerBuild>().owned_bond_cards,
            before
        );
        assert!(app.world().resource::<DraftState>().root.is_some());
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Free)
        );
    }

    #[test]
    fn exhausted_card_pool_never_charges_or_opens_empty_window() {
        let mut app = test_app();
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards
            .fill(true);
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 30.;
            session.wood = 100;
        }
        tap_f(&mut app);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_eq!(app.world().resource::<GameSession>().wood, 100);
        assert!(app.world().resource::<BondEconomy>().free_draw_available);
        app.world_mut()
            .resource_mut::<BondEconomy>()
            .free_draw_available = false;
        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 100);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 0);
    }

    #[test]
    fn replacement_later_preserves_payment_and_only_removes_selected_family() {
        let mut app = test_app();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 30.;
            session.wood = 90;
            session.bond_draws = 1;
        }
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.owned_bond_cards[BondCardId::DivineMight.index()] = true;
            for card in BondId::Journey.definition().members {
                build.owned_bond_cards[card.index()] = true;
            }
        }
        install_test_bond_draft(
            &mut app,
            vec![DraftChoice::ReplaceBond(BondId::Ascension)],
            Some(BondCardId::JudgeBrush),
            BondDrawPayment::Wood(10),
        );
        press_action(&mut app, DraftAction::Later);
        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 90);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Wood(10))
        );
        press_choice(&mut app, 0);
        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.bond(BondId::Ascension), 0);
        assert!(build.bond_complete(BondId::Journey));
        assert!(build.owns_bond_card(BondCardId::JudgeBrush));
        assert_eq!(app.world().resource::<GameSession>().wood, 90);
    }

    #[test]
    fn abandoning_replacement_preserves_owned_cards_and_grants_refresh() {
        let mut app = test_app();
        app.world_mut()
            .resource_mut::<PlayerBuild>()
            .owned_bond_cards[BondCardId::DivineMight.index()] = true;
        let before = app.world().resource::<PlayerBuild>().owned_bond_cards;
        let rerolls = app.world().resource::<GameSession>().skill_rerolls;
        install_test_bond_draft(
            &mut app,
            vec![DraftChoice::ReplaceBond(BondId::Ascension)],
            Some(BondCardId::JudgeBrush),
            BondDrawPayment::Free,
        );
        press_action(&mut app, DraftAction::Abandon);
        assert_eq!(
            app.world().resource::<PlayerBuild>().owned_bond_cards,
            before
        );
        assert_eq!(
            app.world().resource::<GameSession>().skill_rerolls,
            rerolls + 1
        );
        assert!(
            app.world()
                .resource::<DraftState>()
                .pending_bond_replacement
                .is_none()
        );
    }

    #[test]
    fn compass_guarantees_an_unowned_card_from_highest_unfinished_family() {
        let mut build = PlayerBuild::default();
        for card in [
            BondCardId::JourneyStrength,
            BondCardId::JourneyAgility,
            BondCardId::JourneyIntelligence,
            BondCardId::DivineMight,
            BondCardId::DivineBody,
        ] {
            build.owned_bond_cards[card.index()] = true;
        }
        let target = highest_progress_unfinished_bond(&build);
        assert_eq!(target, Some(BondId::Ascension));
        let choices = pick_bond_choices(
            &mut RunRng::default(),
            &build,
            &SelectedHero::default(),
            &mut DraftAntiDrought::default(),
            target,
        );
        assert!(choices.iter().any(|c|matches!(c,DraftChoice::Bond(card) if card.family()==BondId::Ascension && !build.owns_bond_card(*card))));
    }

    fn autoplay_bond_app(elapsed: f32, wood: u32) -> App {
        let mut app = test_app();
        app.insert_resource(Autoplay);
        // Keep the automatic G draft out of the way: no free point and no gold.
        app.world_mut()
            .resource_mut::<SkillEconomy>()
            .first_free_granted = true;
        let mut session = app.world_mut().resource_mut::<GameSession>();
        session.elapsed = elapsed;
        session.gold = 0;
        session.wood = wood;
        app
    }

    #[test]
    fn autoplay_draws_bonds_back_to_back_whenever_it_can_pay() {
        let mut app = autoplay_bond_app(FIRST_AUTO_BOND_AT, 25);

        app.update();
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Free)
        );
        app.update();
        let session = app.world().resource::<GameSession>();
        assert_eq!(
            (session.wood, session.bond_draws),
            (25, 0),
            "an open draft must not be paid for twice"
        );

        // Chosen cards may grant wood, so each step funds exactly one draw.
        for (paid_draws, price) in [(1, 10), (2, 15)] {
            finish_autoplay_draft_after_1_15_seconds(&mut app);
            app.world_mut().resource_mut::<GameSession>().wood = price;
            app.update();
            let session = app.world().resource::<GameSession>();
            assert_eq!(
                session.elapsed, FIRST_AUTO_BOND_AT,
                "no waiting between draws"
            );
            assert_eq!((session.wood, session.bond_draws), (0, paid_draws));
            assert_eq!(
                app.world().resource::<BondEconomy>().pending_payment,
                Some(BondDrawPayment::Wood(price))
            );
        }

        finish_autoplay_draft_after_1_15_seconds(&mut app);
        app.world_mut().resource_mut::<GameSession>().wood = 19;
        app.update();
        app.update();
        let session = app.world().resource::<GameSession>();
        assert_eq!((session.wood, session.bond_draws), (19, 2));
        assert!(
            app.world()
                .resource::<BondEconomy>()
                .pending_payment
                .is_none(),
            "a draw that cannot be paid for must not open"
        );
    }

    #[test]
    fn autoplay_waits_for_eighteen_seconds_before_its_first_bond_draw() {
        assert_eq!(FIRST_FREE_BOND_AT, 12.0);
        assert_eq!(FIRST_AUTO_BOND_AT, 18.0);
        let mut app = autoplay_bond_app(FIRST_AUTO_BOND_AT - 0.1, 100);
        app.update();
        assert!(app.world().resource::<BondEconomy>().free_draw_available);
        assert!(app.world().resource::<DraftState>().kind.is_none());

        app.world_mut().resource_mut::<GameSession>().elapsed = FIRST_AUTO_BOND_AT;
        app.update();
        assert_eq!(
            app.world().resource::<DraftState>().kind,
            Some(DraftKind::Bond)
        );
    }

    #[test]
    fn autoplay_leaves_bonds_alone_when_auto_bond_draws_is_off() {
        let mut app = autoplay_bond_app(300.0, 100);
        app.world_mut()
            .resource_mut::<GameSession>()
            .auto_bond_draws = false;
        app.update();
        app.update();
        assert!(app.world().resource::<DraftState>().kind.is_none());
        assert_eq!(app.world().resource::<GameSession>().wood, 100);

        tap_f(&mut app);
        assert_eq!(
            app.world().resource::<DraftState>().kind,
            Some(DraftKind::Bond),
            "manual F still works"
        );
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

        app.world_mut().resource_mut::<GameSession>().wood = 100;
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
        assert_eq!(app.world().resource::<GameSession>().wood, 75);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 2);
        press_action(&mut app, DraftAction::Abandon);
        tap_f(&mut app);
        assert_eq!(app.world().resource::<GameSession>().wood, 55);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 3);
        assert_eq!(
            app.world().resource::<BondEconomy>().pending_payment,
            Some(BondDrawPayment::Wood(20))
        );
    }

    #[test]
    fn unique_absorbed_heroes_open_real_training_slots_without_erasing_mastery() {
        let mut app = test_app();
        app.init_resource::<crate::game::HeroFusion>();
        {
            let mut fusion = app.world_mut().resource_mut::<crate::game::HeroFusion>();
            assert!(fusion.record_absorbed(crate::game::HeroKind::StormSavant));
            assert!(fusion.record_absorbed(crate::game::HeroKind::StoneWarden));
            assert!(!fusion.record_absorbed(crate::game::HeroKind::StormSavant));
            fusion.register_starting_skill(crate::game::HeroKind::StormSavant, SkillId(0));
            fusion.register_starting_skill(crate::game::HeroKind::StoneWarden, SkillId(3));
            fusion.register_starting_skill(crate::game::HeroKind::FrostOracle, SkillId(2));
        }
        for rank in &mut app.world_mut().resource_mut::<PlayerBuild>().skills[..6] {
            *rank = 5;
        }
        app.world_mut().resource_mut::<SkillEvolution>().tiers[0] = 2;
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.level = 10;
            session.elapsed = 30.0;
            session.gold = 5000;
        }
        tap_g(&mut app);
        assert_eq!(app.world().resource::<SkillEvolution>().skill_capacity(), 8);
        let choice = app
            .world()
            .resource::<DraftState>()
            .choices
            .iter()
            .position(|choice| matches!(choice,DraftChoice::Skill(skill) if skill.0 >= 6))
            .expect("new fusion slot offers an unowned skill");
        press_choice(&mut app, choice);
        let build = app.world().resource::<PlayerBuild>();
        assert_eq!(build.skills.iter().filter(|rank| **rank > 0).count(), 7);
        assert!(build.skills[..6].iter().all(|rank| *rank == 5));
        assert_eq!(app.world().resource::<SkillEvolution>().tiers[0], 2);
    }
    #[test]
    fn returning_to_an_old_hero_preserves_registered_capacity_and_mastery() {
        use crate::game::{HeroFusion, HeroKind};
        let mut app = test_app();
        app.init_resource::<HeroFusion>();
        app.world_mut().resource_mut::<PlayerBuild>().skills[6] = 5;
        app.world_mut().resource_mut::<SkillEvolution>().tiers[6] = 2;
        let mut previous = None;
        for (kind, skill, capacity) in [
            (HeroKind::StormSavant, SkillId(0), 6),
            (HeroKind::GroveArtificer, SkillId(17), 7),
            (HeroKind::StormSavant, SkillId(12), 7),
            (HeroKind::FrostOracle, SkillId(2), 8),
        ] {
            {
                let mut fusion = app.world_mut().resource_mut::<HeroFusion>();
                if let Some(old) = previous {
                    fusion.record_absorbed(old);
                }
                fusion.register_starting_skill(kind, skill);
            }
            app.update();
            assert_eq!(
                app.world().resource::<HeroFusion>().skill_capacity(),
                capacity
            );
            assert_eq!(
                app.world().resource::<SkillEvolution>().skill_capacity(),
                capacity
            );
            assert_eq!(app.world().resource::<PlayerBuild>().skills[6], 5);
            assert_eq!(app.world().resource::<SkillEvolution>().tiers[6], 2);
            previous = Some(kind);
        }
        assert_eq!(
            app.world()
                .resource::<HeroFusion>()
                .starting_skill(HeroKind::StormSavant),
            Some(SkillId(0))
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
                SKILL_SLOT_LIMIT,
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
    #[test]
    fn collection_stage_requirement_matches_draft_gate_level_or_time() {
        for family in BondId::ALL {
            let Some((level, seconds)) = bond_stage_requirement(family) else {
                continue;
            };
            let card = family.definition().members[0];
            let mut session = GameSession {
                level: level - 1,
                elapsed: seconds - 0.1,
                ..default()
            };
            assert!(!bond_stage_available(card, &session));
            session.level = level;
            assert!(bond_stage_available(card, &session));
            session.level = level - 1;
            session.elapsed = seconds;
            assert!(bond_stage_available(card, &session));
        }
    }
    fn three_remaining_bond_app(wood: u32) -> App {
        let mut app = test_app();
        {
            let mut build = app.world_mut().resource_mut::<PlayerBuild>();
            build.owned_bond_cards.fill(true);
            for card in [
                BondCardId::DivineMight,
                BondCardId::DivineStrength,
                BondCardId::DivineIntelligence,
            ] {
                build.owned_bond_cards[card.index()] = false;
            }
        }
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.elapsed = 30.0;
            session.wood = wood;
        }
        app.world_mut()
            .resource_mut::<BondEconomy>()
            .first_free_granted = true;
        app
    }
    #[test]
    fn one_f_entry_chains_three_real_bond_acquisitions_until_wood_is_exhausted() {
        let mut app = three_remaining_bond_app(45);
        tap_f(&mut app);
        for remaining in [35, 20, 0] {
            let DraftChoice::Bond(card) = app.world().resource::<DraftState>().choices[0] else {
                panic!("expected bond");
            };
            assert_eq!(app.world().resource::<GameSession>().wood, remaining);
            press_first_choice(&mut app);
            assert!(app.world().resource::<PlayerBuild>().owns_bond_card(card));
            app.update();
        }
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 3);
        assert_eq!(app.world().resource::<GameSession>().wood, 0);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert!(app.world().resource::<DraftContinuation>().queued.is_none());
    }
    #[test]
    fn successful_later_and_abandon_stop_continuation_without_another_charge() {
        for action in [DraftAction::Later, DraftAction::Abandon] {
            let mut app = three_remaining_bond_app(100);
            tap_f(&mut app);
            press_action(&mut app, action);
            assert!(!app.world().resource::<DraftContinuation>().enabled);
            let wood = app.world().resource::<GameSession>().wood;
            for _ in 0..3 {
                app.update();
            }
            assert_eq!(app.world().resource::<GameSession>().wood, wood);
            assert!(app.world().resource::<DraftState>().root.is_none());
            assert!(app.world().resource::<DraftContinuation>().queued.is_none());
        }
    }
    #[test]
    fn one_e_entry_drains_three_banked_encounters_through_real_card_clicks() {
        let mut app = test_app();
        {
            let mut encounters = app.world_mut().resource_mut::<EncounterState>();
            for boss in 0..3 {
                assert!(encounters.bank_for_boss(boss));
            }
        }
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.gold = 100_000;
            session.wood = 100_000;
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        for left in [2, 1, 0] {
            assert_eq!(app.world().resource::<EncounterState>().banked, left);
            let (index, event) = app
                .world()
                .resource::<DraftState>()
                .choices
                .iter()
                .copied()
                .enumerate()
                .find_map(|(index, choice)| match choice {
                    DraftChoice::Encounter(event)
                        if event != super::super::encounters::EncounterId::AllIn
                            && event.affordable(app.world().resource::<GameSession>()) =>
                    {
                        Some((index, event))
                    }
                    _ => None,
                })
                .expect("an affordable encounter");
            press_choice(&mut app, index);
            assert!(app.world().resource::<EncounterState>().taken(event));
            app.update();
        }
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert!(!app.world().resource::<EncounterState>().pending());
        assert_eq!(app.world().resource::<EncounterState>().banked, 0);
    }
    #[test]
    fn one_g_entry_consumes_free_first_then_two_paid_draws_until_gold_is_exhausted() {
        let mut app = test_app();
        {
            let mut session = app.world_mut().resource_mut::<GameSession>();
            session.skill_points = 1;
            session.gold = 175;
        }
        let ranks = app
            .world()
            .resource::<PlayerBuild>()
            .skills
            .iter()
            .map(|rank| u32::from(*rank))
            .sum::<u32>();
        tap_g(&mut app);
        for gold in [175, 95, 0] {
            assert_eq!(app.world().resource::<GameSession>().gold, gold);
            press_first_choice(&mut app);
            app.update();
        }
        assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
        assert_eq!(app.world().resource::<GameSession>().skill_points, 0);
        assert!(app.world().resource::<DraftState>().root.is_none());
        assert_eq!(
            app.world()
                .resource::<PlayerBuild>()
                .skills
                .iter()
                .map(|rank| u32::from(*rank))
                .sum::<u32>(),
            ranks + 3
        );
        assert!(app.world().resource::<DraftContinuation>().queued.is_none());
    }
    #[test]
    fn successful_refresh_and_failed_refresh_preserve_paid_skill_continuation() {
        for rerolls in [0, 1] {
            let mut app = test_app();
            {
                let mut session = app.world_mut().resource_mut::<GameSession>();
                session.gold = 175;
                session.skill_rerolls = rerolls;
            }
            tap_g(&mut app);
            assert_eq!(app.world().resource::<GameSession>().gold, 95);
            press_action(&mut app, DraftAction::Refresh);
            assert!(app.world().resource::<DraftContinuation>().enabled);
            assert_eq!(app.world().resource::<GameSession>().gold, 95);
            assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 1);
            press_first_choice(&mut app);
            app.update();
            assert_eq!(app.world().resource::<GameSession>().gold, 0);
            assert_eq!(app.world().resource::<SkillEconomy>().paid_draws, 2);
            assert!(app.world().resource::<DraftState>().root.is_some());
            press_first_choice(&mut app);
            app.update();
            assert!(app.world().resource::<DraftState>().root.is_none());
        }
    }
    #[test]
    fn bond_refresh_preserves_chain_and_failed_encounter_abandon_does_not_cancel_banked_claims() {
        let mut app = three_remaining_bond_app(45);
        tap_f(&mut app);
        press_action(&mut app, DraftAction::Refresh);
        assert!(app.world().resource::<DraftContinuation>().enabled);
        press_first_choice(&mut app);
        app.update();
        assert_eq!(app.world().resource::<GameSession>().wood, 20);
        assert_eq!(app.world().resource::<GameSession>().bond_draws, 2);
        let mut app = test_app();
        {
            let mut encounters = app.world_mut().resource_mut::<EncounterState>();
            encounters.bank_for_boss(0);
            encounters.bank_for_boss(1);
        }
        app.world_mut().resource_mut::<GameSession>().gold = 100_000;
        app.world_mut().resource_mut::<GameSession>().wood = 100_000;
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyE);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        let action = app
            .world_mut()
            .spawn((
                DraftActionButton(DraftAction::Abandon),
                Interaction::Pressed,
            ))
            .id();
        app.update();
        assert!(app.world().resource::<DraftContinuation>().enabled);
        assert_eq!(app.world().resource::<EncounterState>().banked, 1);
        app.world_mut().entity_mut(action).despawn();
        press_first_choice(&mut app);
        app.update();
        assert_eq!(app.world().resource::<EncounterState>().banked, 0);
        assert!(app.world().resource::<DraftState>().root.is_some());
    }
}

#[cfg(test)]
mod bond_draw_sim {
    //! `cargo test --lib bond_draw_sim -- --ignored --nocapture`
    use super::*;

    /// Picks the card whose family is closest to completion (a player chasing sets).
    fn completer_pick(choices: &[DraftChoice], build: &PlayerBuild) -> BondCardId {
        choices
            .iter()
            .filter_map(|choice| match choice {
                DraftChoice::Bond(card) => Some(*card),
                _ => None,
            })
            .max_by_key(|card| {
                let family = card.family();
                let missing = family.required() - build.bond(family);
                (build.bond(family) > 0, std::cmp::Reverse(missing))
            })
            .expect("bond hand")
    }

    pub(super) fn run(seed: u64, draws: u32) -> (Vec<u32>, Vec<BondId>, u32, u32) {
        let mut rng = RunRng::from_seed(seed);
        let mut build = PlayerBuild::default();
        let selected = SelectedHero {
            kind: super::super::HeroKind::ALL[(seed % 12) as usize],
            locked: true,
        };
        let mut drought = DraftAntiDrought::default();
        let mut session = GameSession::default();
        let mut completed_at = Vec::new();
        let mut hands_with_existing = 0;
        let mut hands = 0;
        for draw in 0..draws {
            // ~one draw every 40 s of a 25 min run, level ~ 1.6 per minute.
            session.elapsed = draw as f32 * 40.0;
            session.level = 1 + (session.elapsed / 37.5) as u32;
            let choices =
                pick_bond_choices_at(&mut rng, &build, &selected, &mut drought, None, &session);
            if choices.is_empty() {
                break;
            }
            if build.bond_complete(BondId::Journey) {
                hands += 1;
                if choices.iter().any(|choice| matches!(choice, DraftChoice::Bond(card) if build.bond(card.family()) > 0)) {
                    hands_with_existing += 1;
                }
            }
            let card = completer_pick(&choices, &build);
            let before = build.bond_complete(card.family());
            build.owned_bond_cards[card.index()] = true;
            if !before && build.bond_complete(card.family()) && card.family() != BondId::Journey {
                completed_at.push(draw + 1);
            }
        }
        let completed = BondId::ALL
            .into_iter()
            .filter(|family| *family != BondId::Journey && build.bond_complete(*family))
            .collect();
        (completed_at, completed, hands_with_existing, hands)
    }

    fn affordable_draws(wood: u32) -> u32 {
        let mut session = GameSession::default();
        let mut remaining = wood;
        let encounters = EncounterState::default();
        // First draw is free.
        let mut draws = 1;
        while remaining >= bond_cost(&session, &encounters) {
            remaining -= bond_cost(&session, &encounters);
            session.bond_draws += 1;
            draws += 1;
        }
        draws
    }

    #[test]
    #[ignore]
    fn report_bond_completion_pace() {
        for wood in [800, 1_600, 2_400] {
            let draws = affordable_draws(wood);
            let seeds = 200;
            let mut total = 0usize;
            let mut sets = std::collections::BTreeSet::new();
            let mut existing = 0;
            let mut hands = 0;
            for seed in 1..=seeds {
                let (done, completed, e, h) = run(seed, draws);
                total += done.len();
                sets.insert(
                    completed
                        .iter()
                        .map(|family| family.index())
                        .collect::<Vec<_>>(),
                );
                existing += e;
                hands += h;
            }
            println!(
                "wood={wood:>5} draws={draws:>2} avg families completed={:.1}/25  distinct completed sets={}/{seeds}  hands with a started family={:.0}%",
                total as f32 / seeds as f32,
                sets.len(),
                100.0 * existing as f32 / hands.max(1) as f32
            );
        }
    }
}
