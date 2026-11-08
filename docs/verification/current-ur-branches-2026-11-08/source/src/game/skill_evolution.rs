//! Three finite post-mastery stages; this is an original progression extension.
pub use super::ur_evolution::UrEvolutionPath;
use super::{BondId, GameLanguage, GameSession, PlayerBuild, SkillId, TrialState};
use bevy::prelude::*;
#[derive(Resource, Default)]
pub struct SkillEvolution {
    pub tiers: [u8; SkillId::COUNT],
    pub paths: [Option<UrEvolutionPath>; SkillId::COUNT],
    /// Historical rank rewards cannot be farmed by replacing and relearning.
    pub(crate) rewarded_ranks: [u8; SkillId::COUNT],
    pub unlocked_stage: u8,
    pub(crate) training_capacity: usize,
}
pub(crate) fn refresh_access(
    session: Res<GameSession>,
    build: Res<PlayerBuild>,
    trial: Option<Res<TrialState>>,
    mut evolution: ResMut<SkillEvolution>,
    fusion: Option<Res<super::HeroFusion>>,
) {
    evolution.training_capacity = fusion.map_or(6, |fusion| fusion.skill_capacity());
    let completed = BondId::ALL
        .into_iter()
        .filter(|id| build.bond_complete(*id))
        .count();
    let depth = trial.map_or(0, |trial| trial.level);
    evolution.unlocked_stage = (1..=3)
        .filter(|tier| {
            session.level >= 20 + 10 * u32::from(*tier)
                && (completed > usize::from(*tier) || depth >= 4 * *tier)
        })
        .max()
        .unwrap_or(0);
    for (index, rank) in build.skills.iter().enumerate() {
        evolution.rewarded_ranks[index] = evolution.rewarded_ranks[index].max(*rank);
    }
}
impl SkillEvolution {
    pub fn skill_capacity(&self) -> usize {
        self.training_capacity.max(6)
    }
    pub fn branch_ready(&self, skill: SkillId, build: &PlayerBuild) -> bool {
        skill.definition().rarity == super::content::Rarity::Ur
            && build.skill_rank(skill) >= skill.definition().max_rank
            && self.paths[usize::from(skill.0)].is_none()
    }
    pub fn eligible(&self, skill: SkillId, build: &PlayerBuild) -> bool {
        let index = usize::from(skill.0);
        skill.definition().rarity != super::content::Rarity::Ur
            && build.skills[index] >= skill.definition().max_rank
            && self.tiers[index] < self.unlocked_stage
    }
}
pub fn description(language: GameLanguage, skill: SkillId, tier: u8) -> String {
    use super::bonds::{DamageElement, DamageProfile};
    let mechanism = match skill.0 {
        6 => language.pick(
            "Additional blades follow alternating inner and outer orbits.",
            "增加环刃，交替沿内外双轨道切割敌人。",
        ),
        4 => language.pick(
            "Bone sentries merge into one giant; its attacks cleave nearby foes.",
            "骷髅哨兵合体为巨像，攻击劈砍周围敌人。",
        ),
        16 => language.pick(
            "The water elemental grows and sends piercing tidal surges through foes.",
            "水元素巨大化，发出贯穿敌群的潮汐冲击。",
        ),
        _ => match DamageProfile::for_skill(usize::from(skill.0)).element {
            DamageElement::Flame => language.pick(
                "Periodically focus fire into a piercing flame beam.",
                "定期聚火成贯穿敌阵的烈焰射线。",
            ),
            DamageElement::Frost => language.pick(
                "Periodically freeze a wider formation with an ice eruption.",
                "定期引爆冰阵，冻结更大范围的敌人。",
            ),
            DamageElement::Storm => language.pick(
                "Periodically discharge a chain across additional enemies.",
                "定期释放连锁电弧，传导至更多敌人。",
            ),
            DamageElement::Tide => language.pick(
                "Periodically drive a tidal fan through enemies and push them back.",
                "定期释放贯穿潮汐扇面，击退敌人。",
            ),
            DamageElement::Physical => language.pick(
                "Periodically release a sweeping cleave across a formation.",
                "定期释放横扫斩击，劈开成片敌群。",
            ),
            _ => language.pick(
                "Periodically release homing echoes at several separate targets.",
                "定期释放追踪回响，追击多个独立目标。",
            ),
        },
    };
    format!(
        "{} {tier}/3\n{mechanism}\n{}",
        language.pick("ASCENSION", "升华"),
        language.pick(
            "Stages: level 30 / 40 / 50; complete 2 / 3 / 4 bonds or clear tower 4 / 8 / 12.",
            "阶段要求：等级30／40／50；完成2／3／4套羁绊，或通过塔4／8／12层。"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stages_require_level_and_actual_bond_or_tower_progress() {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<TrialState>()
            .init_resource::<SkillEvolution>()
            .add_systems(Update, refresh_access);
        app.world_mut().resource_mut::<GameSession>().level = 50;
        app.update();
        assert_eq!(app.world().resource::<SkillEvolution>().unlocked_stage, 0);
        for (level, depth, stage) in [(29, 12, 0), (30, 4, 1), (40, 8, 2), (50, 12, 3)] {
            app.world_mut().resource_mut::<GameSession>().level = level;
            app.world_mut().resource_mut::<TrialState>().level = depth;
            app.update();
            assert_eq!(
                app.world().resource::<SkillEvolution>().unlocked_stage,
                stage
            );
        }
    }
}
