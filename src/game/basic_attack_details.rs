//! Player-facing numbers read from the same class profile as native and inherited attacks.
use super::basic_attacks::{
    BasicWeapon, RICOCHET_DAMAGE_RATIO, RICOCHET_RANGE, class_basic_stats, weapon,
};
use super::{GameLanguage, HeroKind, PlayerBuild};

pub(crate) fn summary(
    language: GameLanguage,
    kind: HeroKind,
    level: u32,
    build: &PlayerBuild,
    cleave_bonus: usize,
) -> String {
    let mut stats = class_basic_stats(kind, level, build);
    if weapon(kind).melee() {
        stats.cleave_targets += cleave_bonus;
    }
    match weapon(kind) {
        BasicWeapon::Crossbow => {
            if language.is_chinese() {
                format!("齐射 {} 箭 · 弹射 {} 次", stats.shots, stats.bounces)
            } else {
                format!("{} arrows · {} bounces", stats.shots, stats.bounces)
            }
        }
        BasicWeapon::Sword | BasicWeapon::Axe => {
            if language.is_chinese() {
                format!(
                    "劈砍 {} 目标 · 附伤 {:.0}%",
                    stats.cleave_targets,
                    stats.cleave_ratio * 100.0
                )
            } else {
                format!(
                    "Cleave {} targets · {:.0}% splash",
                    stats.cleave_targets,
                    stats.cleave_ratio * 100.0
                )
            }
        }
        BasicWeapon::Daggers => {
            if language.is_chinese() {
                format!("刺击 {} 目标 · {:.1} 米", stats.cleave_targets, stats.range)
            } else {
                format!(
                    "{} targets · {:.1}m strike",
                    stats.cleave_targets, stats.range
                )
            }
        }
        _ => {
            if language.is_chinese() {
                format!("{} 发投射物 · {:.1} 米", stats.shots, stats.range)
            } else {
                format!("{} projectiles · {:.1}m", stats.shots, stats.range)
            }
        }
    }
}
pub(crate) fn detail(
    language: GameLanguage,
    kind: HeroKind,
    level: u32,
    build: &PlayerBuild,
    hit_damage: f32,
    cleave_bonus: usize,
) -> String {
    let stats = class_basic_stats(kind, level, build);
    let header = summary(language, kind, level, build, cleave_bonus);
    match weapon(kind) {
        BasicWeapon::Crossbow => {
            if language.is_chinese() {
                format!(
                    "{header}\n物理伤害 {hit_damage:.0}／箭 · 射程 {:.1} 米 · 穿透 {} 次\n弹射距离 {RICOCHET_RANGE:.1} 米 · 每跳保留 {:.0}% 伤害 · 首跳 {:.0}",
                    stats.range,
                    build.pierce,
                    RICOCHET_DAMAGE_RATIO * 100.0,
                    hit_damage * RICOCHET_DAMAGE_RATIO
                )
            } else {
                format!(
                    "{header}\nPhysical {hit_damage:.0}/arrow · {:.1}m range · {} pierces\n{RICOCHET_RANGE:.1}m bounce range · {:.0}% damage retained per hop · first hop {:.0}",
                    stats.range,
                    build.pierce,
                    RICOCHET_DAMAGE_RATIO * 100.0,
                    hit_damage * RICOCHET_DAMAGE_RATIO
                )
            }
        }
        BasicWeapon::Sword | BasicWeapon::Axe => {
            if language.is_chinese() {
                format!(
                    "{header}\n物理伤害：主目标 {hit_damage:.0} · 额外目标 {:.0}\n扇形范围 {:.1} 米／{:.0}°",
                    hit_damage * stats.cleave_ratio,
                    stats.range,
                    (stats.half_angle * 2.0).to_degrees()
                )
            } else {
                format!(
                    "{header}\nPhysical: primary {hit_damage:.0} · additional targets {:.0}\n{:.1}m / {:.0}° forward cone",
                    hit_damage * stats.cleave_ratio,
                    stats.range,
                    (stats.half_angle * 2.0).to_degrees()
                )
            }
        }
        _ => {
            if language.is_chinese() {
                format!("{header}\n物理伤害 {hit_damage:.0}／命中")
            } else {
                format!("{header}\nPhysical {hit_damage:.0}/hit")
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archer_and_warrior_text_tracks_real_level_and_equipment() {
        let build = PlayerBuild {
            multishot: 2,
            ricochet: 1,
            cleave: 2,
            ..Default::default()
        };
        let archer = class_basic_stats(HeroKind::EmberRanger, 20, &build);
        let en = detail(
            GameLanguage::English,
            HeroKind::EmberRanger,
            20,
            &build,
            120.0,
            0,
        );
        assert!(en.contains(&format!(
            "{} arrows · {} bounces",
            archer.shots, archer.bounces
        )));
        assert!(en.contains("first hop 90"));
        assert!(!en.contains("伤害"));
        let warrior = class_basic_stats(HeroKind::StoneWarden, 40, &build);
        let zh = detail(
            GameLanguage::SimplifiedChinese,
            HeroKind::StoneWarden,
            40,
            &build,
            100.0,
            3,
        );
        assert!(zh.contains(&format!("劈砍 {} 目标", warrior.cleave_targets + 3)));
        assert!(zh.contains("额外目标 90"));
        assert!(!zh.contains("Physical"));
    }
}
