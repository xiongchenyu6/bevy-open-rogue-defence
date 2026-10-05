//! Encounters (game design 16.4): each checkpoint boss banks one free three-way trade-off.
//! Every event has a benefit and a cost; a run never offers the same event twice.
use bevy::prelude::*;

use super::content::SKILLS;
use super::{BondId, GameLanguage, GameSession, HERO_TOTAL_LIVES, HeroVitals, PlayerBuild, RunRng};

/// Bosses 0..=3 (5/10/15/20 minutes) bank an encounter; the 25:00 boss does not.
pub const ENCOUNTER_BOSS_COUNT: u8 = 4;
pub const ENCOUNTER_HAND_SIZE: usize = 3;
pub const BLOOD_OATH_DAMAGE_BONUS: f32 = 0.12;
pub const BLOOD_OATH_HEALTH_MULTIPLIER: f32 = 0.85;
pub const IRON_BULWARK_GATE_HP: f32 = 400.0;
pub const IRON_BULWARK_SPEED_MULTIPLIER: f32 = 0.92;
pub const ALL_IN_GOLD_PER_POINT: u32 = 100;
pub const ALL_IN_ATTRIBUTE_CAP: u32 = 40;
pub const TIMBER_LEVY_WOOD: u32 = 150;
pub const TIMBER_LEVY_DRAWS: u8 = 3;
pub const TIMBER_LEVY_PRICE_MULTIPLIER: f32 = 1.5;
pub const BOUNTY_SECONDS: f32 = 120.0;
pub const BOUNTY_ENEMY_DAMAGE_MULTIPLIER: f32 = 1.2;
pub const ELITE_HUNT_COUNT: u8 = 3;
pub const ELITE_HUNT_WOOD: u32 = 60;
pub const ELITE_HUNT_HEALTH_MULTIPLIER: f32 = 1.4;
pub const TEMPERING_ENERGY_MULTIPLIER: f32 = 0.85;
pub const BAPTISM_WOOD_PER_CARD: u32 = 40;
pub const FORCED_MARCH_LEAD_SECONDS: f32 = 30.0;
pub const FORCED_MARCH_REWARD_MULTIPLIER: f32 = 1.5;
pub const TREASURE_MAP_REWARD_MULTIPLIER: f32 = 1.3;
pub const TREASURE_MAP_HEALTH_MULTIPLIER: f32 = 1.2;
pub const ARMORY_DONATION_COST: u32 = 1_500;
pub const ARMORY_DONATION_COOLDOWN_MULTIPLIER: f32 = 0.94;
pub const FATE_REROLL_REFRESHES: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterId {
    BloodOath,
    IronBulwark,
    AllIn,
    TimberLevy,
    Bounty,
    EliteHunt,
    Tempering,
    Baptism,
    ForcedMarch,
    TreasureMap,
    ArmoryDonation,
    FateReroll,
}

impl EncounterId {
    pub const COUNT: usize = 12;
    pub const ALL: [Self; Self::COUNT] = [
        Self::BloodOath,
        Self::IronBulwark,
        Self::AllIn,
        Self::TimberLevy,
        Self::Bounty,
        Self::EliteHunt,
        Self::Tempering,
        Self::Baptism,
        Self::ForcedMarch,
        Self::TreasureMap,
        Self::ArmoryDonation,
        Self::FateReroll,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }

    /// Existing UI art; encounters carry no art of their own.
    pub const fn icon(self) -> &'static str {
        match self {
            Self::BloodOath => "blood_array",
            Self::IronBulwark => "tower",
            Self::AllIn => "gold",
            Self::TimberLevy => "wood",
            Self::Bounty => "skull",
            Self::EliteHunt => "hunter",
            Self::Tempering => "skill",
            Self::Baptism => "bond",
            Self::ForcedMarch => "dodge",
            Self::TreasureMap => "market",
            Self::ArmoryDonation => "shield",
            Self::FateReroll => "refresh",
        }
    }

    pub const fn name(self, language: GameLanguage) -> &'static str {
        match self {
            Self::BloodOath => language.pick("Blood Oath", "血誓"),
            Self::IronBulwark => language.pick("Iron Bulwark", "铁壁"),
            Self::AllIn => language.pick("All In", "豪赌"),
            Self::TimberLevy => language.pick("Timber Levy", "伐木令"),
            Self::Bounty => language.pick("Bounty", "悬赏"),
            Self::EliteHunt => language.pick("Elite Hunt", "精英猎场"),
            Self::Tempering => language.pick("Tempering", "淬炼"),
            Self::Baptism => language.pick("Baptism", "洗礼"),
            Self::ForcedMarch => language.pick("Forced March", "急行军"),
            Self::TreasureMap => language.pick("Treasure Map", "藏宝图"),
            Self::ArmoryDonation => language.pick("Armory Donation", "军械捐赠"),
            Self::FateReroll => language.pick("Fate Reroll", "命运刷新"),
        }
    }

    pub const fn effect(self, language: GameLanguage) -> &'static str {
        match self {
            Self::BloodOath => language.pick("Damage dealt +12%", "造成伤害 +12%"),
            Self::IronBulwark => language.pick(
                "Gate max HP +400, fully repaired",
                "城门最大耐久 +400 并修满",
            ),
            Self::AllIn => language.pick(
                "+1 STR / AGI / INT per 100 gold lost (max +40 each)",
                "每失去 100 金币，力量 / 敏捷 / 智力各 +1（最多 +40）",
            ),
            Self::TimberLevy => language.pick("+150 wood now", "立即获得 150 木材"),
            Self::Bounty => {
                language.pick("Kill credit doubled for 120 s", "120 秒内击杀的杀敌数翻倍")
            }
            Self::EliteHunt => language.pick(
                "Next 3 elites drop +60 wood",
                "接下来 3 只精英额外掉落 60 木材",
            ),
            Self::Tempering => language.pick(
                "A random learned skill +1 rank (may pass the cap by 1)",
                "随机一个已学技能 +1 级（可超上限 1 级）",
            ),
            Self::Baptism => language.pick(
                "Remove the least-progressed unfinished bond; refund 40 wood per card",
                "移除进度最低的未完成羁绊，每张卡返还 40 木材",
            ),
            Self::ForcedMarch => language.pick("Next boss rewards ×1.5", "下一个首领奖励 ×1.5"),
            Self::TreasureMap => language.pick("Gold challenge rewards +30%", "金币挑战奖励 +30%"),
            Self::ArmoryDonation => language.pick("All skill cooldowns −6%", "全部技能冷却 −6%"),
            Self::FateReroll => language.pick("+3 refreshes", "刷新次数 +3"),
        }
    }

    pub const fn cost(self, language: GameLanguage) -> &'static str {
        match self {
            Self::BloodOath => language.pick("Max health −15%", "最大生命 −15%"),
            Self::IronBulwark => language.pick("Move speed −8%", "移动速度 −8%"),
            Self::AllIn => language.pick("Lose all gold", "失去全部金币"),
            Self::TimberLevy => language.pick(
                "Next 3 bond draws cost +50%",
                "接下来 3 次羁绊抽取价格 +50%",
            ),
            Self::Bounty => language.pick("Enemy damage +20% meanwhile", "期间敌人伤害 +20%"),
            Self::EliteHunt => {
                language.pick("Those elites spawn with +40% health", "这些精英生命 +40%")
            }
            Self::Tempering => language.pick("Max energy −15%", "最大能量 −15%"),
            Self::Baptism => {
                language.pick("Its cards and effects are lost", "失去该羁绊的卡片与效果")
            }
            Self::ForcedMarch => language.pick("It arrives 30 s early", "它提前 30 秒出现"),
            Self::TreasureMap => language.pick("Its targets have +20% health", "其目标生命 +20%"),
            Self::ArmoryDonation => language.pick("Costs 1,500 gold", "花费 1,500 金币"),
            Self::FateReroll => language.pick(
                "Lose 1 life (keeps at least 1)",
                "失去 1 条命（至少保留 1 条）",
            ),
        }
    }

    /// Gold the player must hold to accept this event; shown disabled otherwise.
    pub const fn gold_cost(self) -> u32 {
        match self {
            Self::ArmoryDonation => ARMORY_DONATION_COST,
            _ => 0,
        }
    }

    pub fn affordable(self, session: &GameSession) -> bool {
        session.gold >= self.gold_cost()
    }

    /// Events whose benefit has nothing to act on stay out of the hand.
    pub fn offerable(self, build: &PlayerBuild) -> bool {
        match self {
            Self::Tempering => temperable_skills(build).next().is_some(),
            Self::Baptism => least_progressed_unfinished_bond(build).is_some(),
            _ => true,
        }
    }
}

fn temperable_skills(build: &PlayerBuild) -> impl Iterator<Item = usize> + '_ {
    build
        .skills
        .iter()
        .enumerate()
        .filter(|(index, rank)| **rank > 0 && **rank <= SKILLS[*index].max_rank)
        .map(|(index, _)| index)
}

fn least_progressed_unfinished_bond(build: &PlayerBuild) -> Option<BondId> {
    BondId::ALL
        .into_iter()
        .filter(|family| build.bond(*family) > 0 && !build.bond_complete(*family))
        .min_by_key(|family| (build.bond(*family), family.index()))
}

fn scale(value: u32, multiplier: f32) -> u32 {
    (value as f32 * multiplier).round() as u32
}

/// Per-run encounter state: the bank, taken events and every live trade-off.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct EncounterState {
    pub banked: u8,
    credited_bosses: u8,
    taken: [bool; EncounterId::COUNT],
    /// An open hand already consumed one banked encounter and awaits a choice.
    pending: bool,
    outgoing_damage_bonus: f32,
    bounty_remaining: f32,
    levied_bond_draws: u8,
    hunted_elites: u8,
    boss_lead_seconds: f32,
    marched_boss: Option<u8>,
    treasure_map: bool,
    cooldown_multiplier: f32,
}

impl Default for EncounterState {
    fn default() -> Self {
        Self {
            banked: 0,
            credited_bosses: 0,
            taken: [false; EncounterId::COUNT],
            pending: false,
            outgoing_damage_bonus: 0.0,
            bounty_remaining: 0.0,
            levied_bond_draws: 0,
            hunted_elites: 0,
            boss_lead_seconds: 0.0,
            marched_boss: None,
            treasure_map: false,
            cooldown_multiplier: 1.0,
        }
    }
}

impl EncounterState {
    pub fn taken(&self, id: EncounterId) -> bool {
        self.taken[id.index()]
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    /// Credits one encounter per checkpoint boss, once each.
    pub fn bank_for_boss(&mut self, boss_index: u8) -> bool {
        if boss_index >= ENCOUNTER_BOSS_COUNT || self.credited_bosses & (1 << boss_index) != 0 {
            return false;
        }
        self.credited_bosses |= 1 << boss_index;
        self.banked = self.banked.saturating_add(1);
        true
    }

    /// Spends one banked encounter for a fresh hand; `None` when nothing can be offered.
    pub fn open_hand(&mut self, rng: &mut RunRng, build: &PlayerBuild) -> Option<Vec<EncounterId>> {
        if self.banked == 0 || self.pending {
            return None;
        }
        let hand = self.draw_hand(rng, build);
        if hand.is_empty() {
            return None;
        }
        self.banked -= 1;
        self.pending = true;
        Some(hand)
    }

    fn draw_hand(&self, rng: &mut RunRng, build: &PlayerBuild) -> Vec<EncounterId> {
        let mut pool: Vec<EncounterId> = EncounterId::ALL
            .into_iter()
            .filter(|id| !self.taken(*id) && id.offerable(build))
            .collect();
        let mut hand = Vec::with_capacity(ENCOUNTER_HAND_SIZE);
        while hand.len() < ENCOUNTER_HAND_SIZE && !pool.is_empty() {
            let index = rng.next_u32() as usize % pool.len();
            hand.push(pool.swap_remove(index));
        }
        hand
    }

    /// Settles the chosen event on live run state. Returns false when it cannot be
    /// accepted (no open hand, already taken, or unaffordable); nothing changes then.
    pub fn accept(
        &mut self,
        id: EncounterId,
        session: &mut GameSession,
        build: &mut PlayerBuild,
        vitals: &mut HeroVitals,
        rng: &mut RunRng,
    ) -> bool {
        if !self.pending || self.taken(id) || !id.affordable(session) {
            return false;
        }
        match id {
            EncounterId::BloodOath => {
                self.outgoing_damage_bonus += BLOOD_OATH_DAMAGE_BONUS;
                vitals.command_health_multiplier *= BLOOD_OATH_HEALTH_MULTIPLIER;
                vitals.max_health *= BLOOD_OATH_HEALTH_MULTIPLIER;
                vitals.health = vitals.health.min(vitals.max_health);
            }
            EncounterId::IronBulwark => {
                session.core_max_hp += IRON_BULWARK_GATE_HP;
                session.heal_gate(session.core_max_hp);
                build.move_speed *= IRON_BULWARK_SPEED_MULTIPLIER;
            }
            EncounterId::AllIn => {
                let gain = (session.gold / ALL_IN_GOLD_PER_POINT).min(ALL_IN_ATTRIBUTE_CAP) as f32;
                session.gold = 0;
                let primary = vitals.primary_attribute();
                vitals.strength += gain;
                vitals.agility += gain;
                vitals.intelligence += gain;
                let health = gain * 14.0 * vitals.command_health_multiplier;
                vitals.max_health += health;
                vitals.health += health;
                vitals.max_energy += gain * 8.0;
                vitals.energy += gain * 8.0;
                build.attack_damage += vitals.primary_attribute() - primary;
                build.attacks_per_second += gain * 0.028;
                build.move_speed += gain * 0.035;
                build.projectile_speed += gain * 0.08;
                build.attack_range += gain * 0.025;
            }
            EncounterId::TimberLevy => {
                session.wood = session.wood.saturating_add(TIMBER_LEVY_WOOD);
                self.levied_bond_draws = TIMBER_LEVY_DRAWS;
            }
            EncounterId::Bounty => self.bounty_remaining = BOUNTY_SECONDS,
            EncounterId::EliteHunt => self.hunted_elites = ELITE_HUNT_COUNT,
            EncounterId::Tempering => {
                let candidates: Vec<usize> = temperable_skills(build).collect();
                if !candidates.is_empty() {
                    let index = candidates[rng.next_u32() as usize % candidates.len()];
                    build.skills[index] += 1;
                }
                vitals.max_energy *= TEMPERING_ENERGY_MULTIPLIER;
                vitals.energy = vitals.energy.min(vitals.max_energy);
            }
            EncounterId::Baptism => {
                if let Some(family) = least_progressed_unfinished_bond(build) {
                    let cards = u32::from(build.bond(family));
                    super::progression::remove_bond(family, build);
                    session.wood = session.wood.saturating_add(cards * BAPTISM_WOOD_PER_CARD);
                }
            }
            EncounterId::ForcedMarch => self.boss_lead_seconds = FORCED_MARCH_LEAD_SECONDS,
            EncounterId::TreasureMap => self.treasure_map = true,
            EncounterId::ArmoryDonation => {
                session.gold -= ARMORY_DONATION_COST;
                self.cooldown_multiplier *= ARMORY_DONATION_COOLDOWN_MULTIPLIER;
            }
            EncounterId::FateReroll => {
                session.skill_rerolls = session.skill_rerolls.saturating_add(FATE_REROLL_REFRESHES);
                vitals.deaths = (vitals.deaths + 1).min(HERO_TOTAL_LIVES - 1);
            }
        }
        self.taken[id.index()] = true;
        self.pending = false;
        true
    }

    /// Timed windows run on battle time; the caller skips paused frames.
    pub fn tick(&mut self, seconds: f32) {
        self.bounty_remaining = (self.bounty_remaining - seconds.max(0.0)).max(0.0);
    }

    pub fn outgoing_damage_multiplier(&self) -> f32 {
        1.0 + self.outgoing_damage_bonus
    }

    pub fn bounty_active(&self) -> bool {
        self.bounty_remaining > 0.0
    }

    pub fn enemy_damage_multiplier(&self) -> f32 {
        if self.bounty_active() {
            BOUNTY_ENEMY_DAMAGE_MULTIPLIER
        } else {
            1.0
        }
    }

    pub fn kill_credit(&self, base: u32) -> u32 {
        if self.bounty_active() {
            base.saturating_mul(2)
        } else {
            base
        }
    }

    pub fn bond_price(&self, base: u32) -> u32 {
        if self.levied_bond_draws > 0 {
            scale(base, TIMBER_LEVY_PRICE_MULTIPLIER)
        } else {
            base
        }
    }

    /// Called once per paid bond draw.
    pub fn consume_bond_levy(&mut self) {
        self.levied_bond_draws = self.levied_bond_draws.saturating_sub(1);
    }

    /// Claims one hunted elite slot for an elite about to spawn.
    pub fn take_hunted_elite(&mut self) -> bool {
        if self.hunted_elites == 0 {
            return false;
        }
        self.hunted_elites -= 1;
        true
    }

    pub fn boss_lead_seconds(&self) -> f32 {
        self.boss_lead_seconds
    }

    /// The first boss spawned under a forced march carries its reward bonus.
    pub fn note_boss_spawned(&mut self, boss_index: u8) {
        if self.boss_lead_seconds > 0.0 {
            self.boss_lead_seconds = 0.0;
            self.marched_boss = Some(boss_index);
        }
    }

    pub fn boss_reward_multiplier(&self, boss_index: u8) -> f32 {
        if self.marched_boss == Some(boss_index) {
            FORCED_MARCH_REWARD_MULTIPLIER
        } else {
            1.0
        }
    }

    pub fn boss_reward(&self, boss_index: u8, base: u32) -> u32 {
        scale(base, self.boss_reward_multiplier(boss_index))
    }

    pub fn gold_challenge_health_multiplier(&self) -> f32 {
        if self.treasure_map {
            TREASURE_MAP_HEALTH_MULTIPLIER
        } else {
            1.0
        }
    }

    pub fn gold_challenge_reward(&self, base: u32) -> u32 {
        if self.treasure_map {
            scale(base, TREASURE_MAP_REWARD_MULTIPLIER)
        } else {
            base
        }
    }

    /// Cooldowns recover faster instead of every cast site shrinking its timer.
    pub fn cooldown_recovery_multiplier(&self) -> f32 {
        1.0 / self.cooldown_multiplier
    }
}

pub struct EncountersPlugin;

impl Plugin for EncountersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EncounterState>()
            .add_systems(Update, tick_encounters);
    }
}

fn tick_encounters(
    time: Res<Time>,
    session: Res<GameSession>,
    mut encounters: ResMut<EncounterState>,
) {
    if session.paused || session.finished {
        return;
    }
    encounters.tick(time.delta_secs());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{BondCardId, HeroKind};

    fn close(left: f32, right: f32) {
        assert!((left - right).abs() < 0.001, "{left} != {right}");
    }

    fn open(state: &mut EncounterState, build: &PlayerBuild) {
        state.banked = 1;
        assert!(state.open_hand(&mut RunRng::default(), build).is_some());
    }

    fn accept(id: EncounterId) -> (EncounterState, GameSession, PlayerBuild, HeroVitals) {
        let mut state = EncounterState::default();
        let mut session = GameSession {
            gold: 2_000,
            ..default()
        };
        let mut build = PlayerBuild::default();
        build.skills[0] = 1;
        let mut vitals = HeroVitals::default();
        open(&mut state, &build);
        assert!(state.accept(
            id,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        assert!(state.taken(id));
        assert!(!state.pending());
        (state, session, build, vitals)
    }

    #[test]
    fn checkpoint_bosses_bank_one_encounter_each_and_the_final_boss_banks_none() {
        let mut state = EncounterState::default();
        for boss in 0..ENCOUNTER_BOSS_COUNT {
            assert!(state.bank_for_boss(boss));
            assert!(!state.bank_for_boss(boss), "a boss credits only once");
            assert_eq!(state.banked, boss + 1);
        }
        assert!(!state.bank_for_boss(4));
        assert_eq!(state.banked, 4);
    }

    #[test]
    fn hand_has_three_distinct_events_never_repeats_taken_ones_and_follows_the_seed() {
        let build = PlayerBuild::default();
        let mut state = EncounterState {
            banked: 4,
            ..default()
        };
        let mut replay_state = state.clone();
        let mut taken = Vec::new();
        let mut rng = RunRng::from_seed(0xA11CE);
        let mut replay = RunRng::from_seed(0xA11CE);
        for _ in 0..4 {
            let hand = state.open_hand(&mut rng, &build).expect("hand");
            assert_eq!(
                hand,
                replay_state
                    .open_hand(&mut replay, &build)
                    .expect("replayed hand"),
                "the same seed draws the same hand"
            );
            assert_eq!(hand.len(), ENCOUNTER_HAND_SIZE);
            for (index, id) in hand.iter().enumerate() {
                assert!(!hand[..index].contains(id), "duplicate in hand");
                assert!(!taken.contains(id), "taken event offered again");
            }
            let mut session = GameSession::default();
            let mut vitals = HeroVitals::default();
            let mut build = PlayerBuild::default();
            let chosen = hand
                .iter()
                .copied()
                .find(|id| id.affordable(&session))
                .expect("an affordable event");
            assert!(state.accept(chosen, &mut session, &mut build, &mut vitals, &mut rng));
            let mut replay_session = GameSession::default();
            assert!(replay_state.accept(
                chosen,
                &mut replay_session,
                &mut PlayerBuild::default(),
                &mut HeroVitals::default(),
                &mut replay
            ));
            taken.push(chosen);
        }
        assert_eq!(state.banked, 0);
        assert!(state.open_hand(&mut rng, &build).is_none());
    }

    #[test]
    fn opening_spends_one_bank_and_a_pending_hand_cannot_be_reopened() {
        let build = PlayerBuild::default();
        let mut state = EncounterState::default();
        assert!(state.open_hand(&mut RunRng::default(), &build).is_none());
        state.banked = 2;
        assert!(state.open_hand(&mut RunRng::default(), &build).is_some());
        assert_eq!(state.banked, 1);
        assert!(state.pending());
        assert!(state.open_hand(&mut RunRng::default(), &build).is_none());
        assert_eq!(state.banked, 1);
    }

    #[test]
    fn accepting_without_an_open_hand_changes_nothing() {
        let mut state = EncounterState::default();
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        let wood = session.wood;
        assert!(!state.accept(
            EncounterId::TimberLevy,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        assert_eq!(session.wood, wood);
        assert!(!state.taken(EncounterId::TimberLevy));
    }

    #[test]
    fn blood_oath_trades_max_health_for_outgoing_damage() {
        let (state, _, _, vitals) = accept(EncounterId::BloodOath);
        close(state.outgoing_damage_multiplier(), 1.12);
        close(vitals.max_health, 800.0 * 0.85);
        close(vitals.health, vitals.max_health);
        close(vitals.command_health_multiplier, 0.85);
    }

    #[test]
    fn iron_bulwark_raises_and_repairs_the_gate_but_slows_the_hero() {
        let mut state = EncounterState::default();
        let mut session = GameSession {
            core_hp: 300.0,
            ..default()
        };
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        let speed = build.move_speed;
        open(&mut state, &build);
        assert!(state.accept(
            EncounterId::IronBulwark,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        close(session.core_max_hp, 1_400.0);
        close(session.core_hp, 1_400.0);
        close(build.move_speed, speed * 0.92);
    }

    #[test]
    fn all_in_converts_every_hundred_gold_into_attributes_with_a_cap() {
        let (_, session, build, vitals) = accept(EncounterId::AllIn);
        assert_eq!(session.gold, 0);
        let base = HeroVitals::default();
        close(vitals.strength, base.strength + 20.0);
        close(vitals.agility, base.agility + 20.0);
        close(vitals.intelligence, base.intelligence + 20.0);
        close(vitals.max_health, base.max_health + 20.0 * 14.0);
        close(vitals.max_energy, base.max_energy + 20.0 * 8.0);
        close(
            build.attack_damage,
            PlayerBuild::default().attack_damage + 20.0,
        );

        let mut state = EncounterState::default();
        let mut session = GameSession {
            gold: 9_950,
            ..default()
        };
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        open(&mut state, &build);
        state.accept(
            EncounterId::AllIn,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default(),
        );
        close(vitals.strength, base.strength + 40.0);
    }

    #[test]
    fn timber_levy_pays_wood_now_and_raises_the_next_three_paid_bond_prices() {
        let (mut state, session, _, _) = accept(EncounterId::TimberLevy);
        assert_eq!(session.wood, 60 + 150);
        for _ in 0..3 {
            assert_eq!(state.bond_price(10), 15);
            state.consume_bond_levy();
        }
        assert_eq!(state.bond_price(10), 10);
    }

    #[test]
    fn bounty_doubles_kill_credit_and_enemy_damage_for_two_battle_minutes() {
        let (mut state, _, _, _) = accept(EncounterId::Bounty);
        assert_eq!(state.kill_credit(3), 6);
        close(state.enemy_damage_multiplier(), 1.2);
        state.tick(119.9);
        assert!(state.bounty_active());
        state.tick(0.1);
        assert_eq!(state.kill_credit(3), 3);
        close(state.enemy_damage_multiplier(), 1.0);
    }

    #[test]
    fn elite_hunt_marks_exactly_three_elites() {
        let (mut state, _, _, _) = accept(EncounterId::EliteHunt);
        assert!(state.take_hunted_elite());
        assert!(state.take_hunted_elite());
        assert!(state.take_hunted_elite());
        assert!(!state.take_hunted_elite());
    }

    #[test]
    fn tempering_raises_one_learned_skill_past_the_cap_once_and_cuts_max_energy() {
        let mut state = EncounterState::default();
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        build.skills[2] = 5;
        let mut vitals = HeroVitals::default();
        open(&mut state, &build);
        assert!(state.accept(
            EncounterId::Tempering,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        assert_eq!(build.skills[2], 6);
        assert_eq!(
            build
                .skills
                .iter()
                .map(|rank| u32::from(*rank))
                .sum::<u32>(),
            6
        );
        close(vitals.max_energy, 504.0 * 0.85);
        close(vitals.energy, vitals.max_energy);
        assert!(!EncounterId::Tempering.offerable(&build));
        build.skills[0] = 1;
        assert!(EncounterId::Tempering.offerable(&build));
    }

    #[test]
    fn baptism_removes_the_least_progressed_family_and_refunds_wood_per_card() {
        let mut build = PlayerBuild::default();
        assert!(!EncounterId::Baptism.offerable(&build));
        for card in BondId::Journey.definition().members {
            build.owned_bond_cards[card.index()] = true;
        }
        for card in [
            BondCardId::DivineMight,
            BondCardId::DivineBody,
            BondCardId::StrengthTraining,
        ] {
            build.owned_bond_cards[card.index()] = true;
        }
        assert!(EncounterId::Baptism.offerable(&build));
        let mut state = EncounterState::default();
        let mut session = GameSession::default();
        let mut vitals = HeroVitals::default();
        open(&mut state, &build);
        assert!(state.accept(
            EncounterId::Baptism,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        assert_eq!(build.bond(BondId::Strength), 0);
        assert_eq!(build.bond(BondId::Ascension), 2);
        assert!(build.bond_complete(BondId::Journey));
        assert_eq!(session.wood, 60 + 40);
    }

    #[test]
    fn forced_march_leads_the_next_boss_by_thirty_seconds_and_pays_it_one_and_a_half_times() {
        let (mut state, _, _, _) = accept(EncounterId::ForcedMarch);
        close(state.boss_lead_seconds(), 30.0);
        state.note_boss_spawned(1);
        close(state.boss_lead_seconds(), 0.0);
        assert_eq!(state.boss_reward(1, 250), 375);
        assert_eq!(state.boss_reward(2, 250), 250);
        state.note_boss_spawned(2);
        assert_eq!(state.boss_reward(2, 250), 250);
    }

    #[test]
    fn treasure_map_raises_gold_challenge_rewards_and_target_health() {
        let (state, _, _, _) = accept(EncounterId::TreasureMap);
        assert_eq!(state.gold_challenge_reward(1_000), 1_300);
        close(state.gold_challenge_health_multiplier(), 1.2);
    }

    #[test]
    fn armory_donation_needs_fifteen_hundred_gold_and_speeds_cooldown_recovery() {
        let mut state = EncounterState::default();
        let mut session = GameSession {
            gold: 1_499,
            ..default()
        };
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals::default();
        open(&mut state, &build);
        assert!(!EncounterId::ArmoryDonation.affordable(&session));
        assert!(!state.accept(
            EncounterId::ArmoryDonation,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        assert!(state.pending());
        session.gold = 1_500;
        assert!(state.accept(
            EncounterId::ArmoryDonation,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default()
        ));
        assert_eq!(session.gold, 0);
        close(state.cooldown_recovery_multiplier(), 1.0 / 0.94);
    }

    #[test]
    fn fate_reroll_grants_three_refreshes_and_never_takes_the_last_life() {
        let (_, session, _, vitals) = accept(EncounterId::FateReroll);
        assert_eq!(session.skill_rerolls, 4);
        assert_eq!(vitals.lives_remaining(), 2);

        let mut state = EncounterState::default();
        let mut session = GameSession::default();
        let mut build = PlayerBuild::default();
        let mut vitals = HeroVitals {
            deaths: 2,
            ..HeroVitals::for_hero(HeroKind::StoneWarden, 1)
        };
        open(&mut state, &build);
        state.accept(
            EncounterId::FateReroll,
            &mut session,
            &mut build,
            &mut vitals,
            &mut RunRng::default(),
        );
        assert_eq!(vitals.lives_remaining(), 1);
    }

    #[test]
    fn every_event_has_single_language_copy_and_a_cost_line() {
        for id in EncounterId::ALL {
            for language in GameLanguage::ALL {
                for text in [id.name(language), id.effect(language), id.cost(language)] {
                    assert!(!text.is_empty());
                    assert_eq!(
                        text.chars().any(|c| c as u32 > 0x2FFF),
                        language.is_chinese(),
                        "{id:?}: {text}"
                    );
                }
            }
            assert!(!id.icon().is_empty());
        }
    }

    #[test]
    fn timed_windows_only_advance_on_unpaused_battle_frames() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .add_plugins(EncountersPlugin);
        app.world_mut()
            .resource_mut::<EncounterState>()
            .bounty_remaining = 1.0;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(5));
        app.update();
        assert!(app.world().resource::<EncounterState>().bounty_active());
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(5));
        app.update();
        assert!(!app.world().resource::<EncounterState>().bounty_active());
    }
}
