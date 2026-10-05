//! Automatic damage spells need real basic attack volleys as well as cooldowns.
use super::*;
use crate::game::content::Rarity;

pub(crate) fn attack_charge_requirement(index: usize, rank: u8) -> u64 {
    if !matches!(
        index,
        0 | 2 | 3 | 8 | 9 | 12 | 13 | 14 | 15 | 18 | 19 | 22 | 23 | 24 | 30 | 31
    ) {
        return 0;
    }
    let base: u64 = match SKILLS[index].rarity {
        Rarity::R => 4,
        Rarity::Sr => 5,
        Rarity::Ssr => 6,
        Rarity::Ur => 8,
    };
    base.saturating_sub(rank.saturating_sub(1).min(2) as u64)
        .max(2)
}
pub(crate) fn rank_cooldown(index: usize, rank: u8) -> f32 {
    let multiplier = if attack_charge_requirement(index, rank) > 0 {
        match rank {
            0 | 1 => 1.7,
            2 => 1.5,
            3 => 1.3,
            4 => 1.15,
            _ => 1.0,
        }
    } else {
        1.0
    };
    skill_cooldown(index) * multiplier
}
#[derive(Default)]
pub(super) struct Cadence {
    pub(super) ranks: [u8; SkillId::COUNT],
    last_attacks: [u64; SkillId::COUNT],
    cast_gap_remaining: f32,
    pub(super) casts: [u64; SkillId::COUNT],
}
#[derive(Resource, Default, Debug)]
pub struct SkillCastTelemetry {
    pub total: u64,
    pub casts: [u64; SkillId::COUNT],
}
impl SkillRuntime {
    pub(super) fn cast_ready(&self, index: usize) -> bool {
        let charge = attack_charge_requirement(index, self.cadence.ranks[index].max(1));
        self.cooldowns[index] <= 0.0
            && (charge == 0
                || self.cadence.cast_gap_remaining <= 0.0
                    && self
                        .attack_count
                        .saturating_sub(self.cadence.last_attacks[index])
                        >= charge)
    }
    pub(super) fn begin_cast(&mut self, index: usize) {
        let rank = self.cadence.ranks[index].max(1);
        self.cooldowns[index] = rank_cooldown(index, rank);
        if attack_charge_requirement(index, rank) > 0 {
            self.cadence.last_attacks[index] = self.attack_count;
            self.cadence.cast_gap_remaining += 0.20;
            self.cadence.casts[index] = self.cadence.casts[index].saturating_add(1);
        }
    }
    pub(super) fn advance_cast_gap(&mut self, delta: f32) {
        self.cadence.cast_gap_remaining = (self.cadence.cast_gap_remaining - delta).max(-delta);
    }
    pub(super) fn prepare_cadence(&mut self, skills: &[u8; SkillId::COUNT]) {
        for (index, &rank) in skills.iter().enumerate() {
            if self.cadence.ranks[index] == 0
                && rank > 0
                && attack_charge_requirement(index, rank) > 0
            {
                self.cadence.last_attacks[index] = self.attack_count;
                self.cooldowns[index] = self.cooldowns[index].max(1.0 + (index % 5) as f32 * 0.35);
            }
            if self.cadence.ranks[index] > 0
                && rank > self.cadence.ranks[index]
                && attack_charge_requirement(index, rank) > 0
            {
                self.cooldowns[index] = self.cooldowns[index].min(rank_cooldown(index, rank));
            }
            self.cadence.ranks[index] = rank;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn charges_only_count_real_volleys_and_each_skill_consumes_its_charge() {
        let mut runtime = SkillRuntime::default();
        let mut skills = [0; SkillId::COUNT];
        for i in [0, 2, 3] {
            skills[i] = 1;
        }
        runtime.prepare_cadence(&skills);
        runtime.cooldowns.fill(0.0);
        assert!(!runtime.cast_ready(0));
        apply_resolved_attack_volleys(&mut runtime, 8, |_| {});
        for i in [0, 2] {
            assert!(runtime.cast_ready(i));
            runtime.begin_cast(i);
            runtime.advance_cast_gap(0.20);
        }
        assert!(runtime.cast_ready(3));
        runtime.begin_cast(3);
        runtime.cooldowns[0] = 0.0;
        assert!(!runtime.cast_ready(0));
        assert_eq!(runtime.cadence.casts.iter().sum::<u64>(), 3);
    }
    #[test]
    fn charged_spells_wait_shared_gap_across_frames_not_across_passives() {
        let mut runtime = SkillRuntime::default();
        let mut skills = [0; SkillId::COUNT];
        skills[0] = 1;
        skills[2] = 1;
        skills[4] = 1;
        runtime.prepare_cadence(&skills);
        runtime.cooldowns.fill(0.0);
        apply_resolved_attack_volleys(&mut runtime, 8, |_| {});
        assert!(runtime.cast_ready(0));
        runtime.begin_cast(0);
        assert!(!runtime.cast_ready(2));
        assert!(runtime.cast_ready(4));
        runtime.prepare_cadence(&skills);
        assert!(!runtime.cast_ready(2));
        runtime.advance_cast_gap(0.19);
        assert!(!runtime.cast_ready(2));
        runtime.advance_cast_gap(0.011);
        assert!(runtime.cast_ready(2));
        runtime.begin_cast(2);
        assert!((runtime.cadence.cast_gap_remaining - 0.199).abs() < 0.001);
    }
    #[test]
    fn accelerated_time_catches_up_without_banking_idle_minutes() {
        let mut runtime = SkillRuntime::default();
        let mut skills = [0; SkillId::COUNT];
        for i in [0, 2, 3, 8] {
            skills[i] = 5;
        }
        runtime.prepare_cadence(&skills);
        runtime.cooldowns.fill(0.0);
        apply_resolved_attack_volleys(&mut runtime, 10_000, |_| {});
        for _ in 0..10_000 {
            runtime.advance_cast_gap(1.0 / 30.0);
        }
        assert!(runtime.cast_ready(0));
        runtime.begin_cast(0);
        assert!(
            !runtime.cast_ready(2),
            "idle time must not fund a synchronized burst"
        );
        runtime.advance_cast_gap(32.0 / 60.0);
        for i in [2, 3] {
            assert!(runtime.cast_ready(i));
            runtime.begin_cast(i);
        }
        assert!(!runtime.cast_ready(8));
        let mut fast = SkillRuntime::default();
        fast.prepare_cadence(&skills);
        fast.cooldowns.fill(0.0);
        apply_resolved_attack_volleys(&mut fast, 8, |_| {});
        fast.advance_cast_gap(32.0 / 60.0);
        for i in [0, 2, 3] {
            assert!(fast.cast_ready(i));
            fast.begin_cast(i);
        }
        assert!(
            !fast.cast_ready(8),
            "per-frame credit bounds catch-up to actual elapsed game time"
        );
    }
    #[test]
    fn rank_upgrade_clamps_stale_cooldown_and_new_run_clears_charge_and_casts() {
        let mut runtime = SkillRuntime::default();
        let mut skills = [0; SkillId::COUNT];
        skills[13] = 1;
        runtime.prepare_cadence(&skills);
        runtime.cooldowns[13] = rank_cooldown(13, 1);
        apply_resolved_attack_volleys(&mut runtime, 8, |_| {});
        skills[13] = 5;
        runtime.prepare_cadence(&skills);
        assert_eq!(runtime.cooldowns[13], rank_cooldown(13, 5));
        runtime.cooldowns[13] = 0.0;
        assert!(runtime.cast_ready(13));
        runtime.begin_cast(13);
        runtime = SkillRuntime::default();
        runtime.prepare_cadence(&skills);
        runtime.cooldowns[13] = 0.0;
        assert!(!runtime.cast_ready(13));
        assert_eq!(runtime.cadence.casts, [0; SkillId::COUNT]);
    }
    #[test]
    fn new_skill_never_inherits_stored_charge_and_rank_shortens_cooldown() {
        let mut runtime = SkillRuntime {
            attack_count: 100,
            ..Default::default()
        };
        let mut skills = [0; SkillId::COUNT];
        skills[13] = 1;
        runtime.prepare_cadence(&skills);
        runtime.cooldowns[13] = 0.0;
        assert!(!runtime.cast_ready(13));
        assert!(rank_cooldown(13, 1) > rank_cooldown(13, 5));
        for i in [4, 5, 6, 10, 11, 16, 17, 20, 21, 25, 26, 27, 28, 29] {
            assert_eq!(attack_charge_requirement(i, 1), 0);
        }
    }
}
