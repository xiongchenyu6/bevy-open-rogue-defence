//! One irreversible specialisation per mastered UR skill, retained by hero absorption.
use super::{GameLanguage, SkillId};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UrEvolutionPath {
    Surge,
    Control,
    Sustain,
}
impl UrEvolutionPath {
    pub const ALL: [Self; 3] = [Self::Surge, Self::Control, Self::Sustain];
    pub const fn index(self) -> usize {
        match self {
            Self::Surge => 0,
            Self::Control => 1,
            Self::Sustain => 2,
        }
    }
}
pub fn name(language: GameLanguage, skill: SkillId, path: UrEvolutionPath) -> &'static str {
    let labels = match skill.0 {
        26 => [
            ("Polar Tempest", "极地风暴"),
            ("Frozen Sovereign", "冰封主宰"),
            ("Twin Wyrms", "双龙永冬"),
        ],
        27 => [
            ("Earthbreaker", "裂地泰坦"),
            ("Gravity Sovereign", "引力主宰"),
            ("Gate Guardian", "城门守护"),
        ],
        28 => [
            ("War Engine", "战争引擎"),
            ("Arcane Engine", "秘法引擎"),
            ("Legion Engine", "军团引擎"),
        ],
        29 => [
            ("War Menagerie", "战兽军团"),
            ("Calamity Dice", "灾变骰子"),
            ("Golden Fortune", "黄金命运"),
        ],
        _ => [("Surge", "爆发"), ("Control", "控制"), ("Sustain", "续航")],
    };
    let (en, zh) = labels[path.index()];
    language.pick(en, zh)
}
pub fn description(language: GameLanguage, skill: SkillId, path: UrEvolutionPath) -> String {
    let effects = match skill.0 {
        26 => [
            (
                "Every 3 seconds, a broad frost surge strikes up to six enemies.",
                "每3秒释放寒潮，打击最多6名敌人。",
            ),
            (
                "Every 2 seconds, frost locks a priority target.",
                "每2秒用冰霜锁住一个优先目标。",
            ),
            (
                "Keep up to two long-lived frost wyrms fighting together.",
                "维持最多两条持久冰龙协同攻击。",
            ),
        ],
        27 => [
            (
                "Every 3 seconds, slam enemies within 5.5 metres.",
                "每3秒震击5.5米范围内的敌人。",
            ),
            (
                "Every second, pull enemies from six metres into a crushing centre.",
                "每秒把6米范围内的敌人拖入碾压中心。",
            ),
            (
                "Every 4 seconds in combat, restore 2% health to the hero and gate.",
                "交战时每4秒恢复英雄与城门各2%生命。",
            ),
        ],
        28 => [
            (
                "Every fourth basic attack adds a physical echo at 70% attack damage.",
                "每四次普攻追加70%攻击伤害的物理回响。",
            ),
            (
                "Every 2.5 seconds, chain spell damage across up to four enemies.",
                "每2.5秒向最多4名敌人释放法术连锁。",
            ),
            (
                "Every 3 seconds, up to eight summons release a legion strike.",
                "每3秒由最多8个召唤物释放军团打击。",
            ),
        ],
        29 => [
            (
                "Every 6 seconds, summon two hunting beasts; at most four remain active.",
                "每6秒召唤两只猎兽，同时最多存在4只。",
            ),
            (
                "Every 5 seconds, blast up to six enemies and stun them for one second.",
                "每5秒轰击最多6名敌人并眩晕1秒。",
            ),
            (
                "Every 8 seconds in combat, gain 20 gold and 4 wood.",
                "交战时每8秒获得20金币与4木材。",
            ),
        ],
        _ => [("", ""), ("", ""), ("", "")],
    };
    let (en, zh) = effects[path.index()];
    format!(
        "{}\n{}\n{}",
        name(language, skill, path),
        language.pick(en, zh),
        language.pick(
            "Choose one route for this run. Hero absorption preserves it.",
            "本局选择一条路线，英雄吞噬保留此进化。"
        )
    )
}
