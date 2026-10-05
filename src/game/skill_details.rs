//! Player-facing numbers derive from the same functions as combat.
use super::content::{SkillTrigger, Tag};
use super::skills::{self, SkillRuntime};
use super::{GameLanguage, HeroVitals, PlayerBuild, SkillId};

const EFFECTS_EN: [&str; SkillId::COUNT] = [
    "Chain lightning: 3–5 hits; each hit deals the displayed damage.",
    "A fiery arrow adds a hit plus equal burn damage over 3 seconds; rank 5 triggers every 2 attacks.",
    "Pierces enemies and slows them 30%; rank 3 adds a second lance, rank 5 freezes.",
    "A travelling water wall hits each enemy once and applies wet plus slow.",
    "Bone guards pursue enemies; rank raises attack damage, size and simultaneous cap.",
    "Summon 2–4 pursuing hounds for 10 seconds; displayed damage is each attack.",
    "Maintain 2–4 circling blades; each enemy can be struck once per second.",
    "Every 4 attacks fires 3–5 piercing arrows, each dealing 60% of attack damage.",
    "A travelling star explodes at its target; displayed damage is each explosion.",
    "A cone shockwave damages enemies and stuns them for 0.6 seconds.",
    "Spells add lightning seals; 4 seals explode for the displayed damage.",
    "Scorch enemies around you every 0.5 seconds; displayed damage is each tick.",
    "A lightning zone lasts 4 seconds and pulses every 0.6 seconds; damage is each pulse.",
    "A meteor warns its impact point for 0.8 seconds, then explodes.",
    "Falling ice leaves a 5-second zone; pulses every 0.8 seconds; 3 hits freeze.",
    "Pull ordinary enemies into a vortex for 4 seconds; displayed damage is total, divided into 8 pulses.",
    "A tidal elemental fights for 12 seconds and applies wet; displayed damage is each attack.",
    "A gate-linked turret lasts 16 seconds; stops firing when the gate falls and resumes after rebuilding.",
    "Infect a high-health enemy for 8 seconds and spread up to 2 generations; ticks use attack scaling.",
    "Launch 1–3 homing swords; displayed damage is each sword hit.",
    "Every 5 non-echo spells repeats the last spell; rank raises the copied damage fraction.",
    "A 10-second banner reduces damage taken, speeds attacks and repairs the gate each second.",
    "An explosion can grant extra gold on a kill; damage and reward chance rise with rank.",
    "A blood wheel travels out and back; return hits also apply equal burn damage.",
    "Strike 6–10 targets with heavenly lightning; displayed damage is each strike.",
    "Intercept a lethal hit, heal and explode; this has its own cooldown and does not consume a life.",
    "A frost wyrm fights for 20 seconds; piercing breath accumulates freeze stacks.",
    "An 18-second colossus pulls ordinary enemies and slams every 3 seconds.",
    "Cycle attack, spell and summon boosts every 4 seconds; rank 5 keeps all three active.",
    "Roll lightning, fire rain, healing, gold or hounds; each outcome has its own formula.",
    "A 12 m flame beam lasts 2 seconds and damages enemies every 0.25 seconds.",
    "Mines arm in 0.6 seconds and last 15 seconds; at most 6 active mines.",
];

pub(crate) fn simple(language: GameLanguage, skill: SkillId, current: u8) -> String {
    let next = current.saturating_add(1).min(skill.definition().max_rank);
    let effect = if language.is_chinese() {
        match skill.0 {
            21 => "旗帜持续10秒，提供减伤、攻速与城门修复。",
            25 => "抵消致命伤，恢复生命并爆燃；独立冷却，不消耗生命次数。",
            _ => skill.definition().effect.description,
        }
    } else {
        EFFECTS_EN[skill.0 as usize]
    };
    format!(
        "{} {current} → {next}/{} · {effect}",
        language.pick("Rank", "等级"),
        skill.definition().max_rank
    )
}

fn trigger(language: GameLanguage, skill: SkillId, rank: u8) -> String {
    let i = skill.0 as usize;
    if i == 25 {
        return format!(
            "{} · {} {:.1}s",
            language.pick("On lethal hit", "致命伤触发"),
            language.pick("Cooldown", "冷却"),
            skills::skill_cooldown(i)
        );
    }
    match skill.definition().trigger {
        SkillTrigger::Cooldown => {
            let cooldown = skills::rank_cooldown(i, rank);
            let charge = skills::attack_charge_requirement(i, rank);
            let base = format!("{} {cooldown:.1}s", language.pick("Cooldown", "冷却"));
            if charge == 0 {
                base
            } else {
                format!(
                    "{base} · {charge} {}",
                    language.pick("basic attacks required", "次普攻蓄力")
                )
            }
        }
        SkillTrigger::EveryNthAttack(n) => format!(
            "{} {} {}",
            language.pick("Every", "每"),
            if i == 1 && rank >= 5 { 2 } else { n },
            language.pick("attacks · no cooldown", "次普攻 · 无冷却")
        ),
        SkillTrigger::EveryNthSpell(n) => format!(
            "{} {n} {}",
            language.pick("Every", "每"),
            language.pick("spells · no cooldown", "次法术 · 无冷却")
        ),
        SkillTrigger::Passive => language
            .pick("Passive · no cast cooldown", "被动 · 无施放冷却")
            .into(),
    }
}
fn formula(language: GameLanguage, skill: SkillId) -> String {
    let i = skill.0 as usize;
    if i == 7 {
        return language
            .pick("Formula: 0.60 × attack damage", "公式：0.60 × 攻击力")
            .into();
    }
    if i == 18 {
        return language
            .pick(
                "Formula: 8 × rank + (0.14 + 0.04 × rank) × hero attack",
                "公式：8 × 等级 +（0.14 + 0.04 × 等级）× 英雄攻击力",
            )
            .into();
    }
    let power = skill.definition().effect.power;
    let mut terms = vec![format!(
        "{:.0} + {:.0} × {}",
        power.base,
        power.per_rank,
        language.pick("(rank − 1)", "（等级 − 1）")
    )];
    for (ratio, en, zh) in [
        (power.attack_ratio, "attack", "攻击力"),
        (power.spell_ratio, "spell power", "法术强度"),
        (power.summon_ratio, "summon power", "召唤强度"),
    ] {
        if ratio > 0.0 {
            terms.push(format!("{ratio:.2} × {}", language.pick(en, zh)));
        }
    }
    format!(
        "{} ({}) × {:.2}",
        language.pick("Formula:", "公式："),
        terms.join(" + "),
        skill.definition().rarity.skill_power_multiplier()
    )
}
fn utility(language: GameLanguage, i: usize, rank: u8) -> String {
    let r = f32::from(rank.saturating_sub(1));
    let mult = super::content::SKILLS[i].rarity.skill_power_multiplier();
    match i {
        20 => format!(
            "{} {:.0}%",
            language.pick("Copied spell damage", "复制法术伤害"),
            ((0.35 + 0.10 * r) * mult).min(1.0) * 100.0
        ),
        21 => format!(
            "{} {:.1}%",
            language.pick("Damage reduction", "减伤"),
            ((0.10 + 0.03 * r) * mult).min(0.75) * 100.0
        ),
        28 => format!(
            "{} {:.1}%",
            language.pick("Active category bonus", "生效类别增幅"),
            (0.20 + 0.05 * r) * mult * 100.0
        ),
        29 => format!(
            "{} {:.0}%; {} {:.0}; {} {}",
            language.pick("Heal", "治疗"),
            ((0.12 + 0.03 * r) * mult).min(0.8) * 100.0,
            language.pick("gold", "金币"),
            ((40.0 + 20.0 * r) * mult).round(),
            language.pick("hounds", "灵犬"),
            if rank >= 4 { 3 } else { 2 }
        ),
        _ => String::new(),
    }
}

pub(crate) fn detailed(
    language: GameLanguage,
    skill: SkillId,
    current: u8,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    runtime: &SkillRuntime,
) -> String {
    let i = skill.0 as usize;
    let rank = current.max(1);
    let next = current.saturating_add(1).min(skill.definition().max_rank);
    let current_value = skills::skill_preview_damage(i, rank, build, vitals, runtime);
    let next_value = skills::skill_preview_damage(i, next, build, vitals, runtime);
    let mut lines = vec![
        format!(
            "{} {current} → {next}/{}",
            language.pick("Rank", "等级"),
            skill.definition().max_rank
        ),
        format!(
            "{} → {}",
            trigger(language, skill, rank),
            trigger(language, skill, next)
        ),
    ];
    if let (Some(a), Some(b)) = (current_value, next_value) {
        let physical = i == 7
            || i == 18
            || i == 23
            || super::bonds::DamageProfile::for_skill(i).element
                == super::bonds::DamageElement::Physical;
        let kind = if i == 18 {
            language.pick("physical infection tick", "物理疫蚀每跳")
        } else if skill.definition().tags.contains(&Tag::Summon) {
            if physical {
                language.pick("physical summon hit", "物理召唤每击")
            } else {
                language.pick("magical summon hit", "魔法召唤每击")
            }
        } else if physical {
            language.pick("physical", "物理")
        } else {
            language.pick("magical", "魔法")
        };
        lines.push(format!(
            "{} ({kind}): {}{a:.1} → {b:.1}",
            language.pick("Damage", "伤害"),
            if current == 0 {
                language.pick("unlearned; rank 1 ", "未学习；1级 ")
            } else {
                ""
            }
        ));
        lines.push(formula(language, skill));
        lines.push(format!(
            "{} {:.1} · {} {:.1} · {} {:.1}",
            language.pick("Attack", "攻击力"),
            build.attack_damage,
            language.pick("Spell power", "法术强度"),
            skills::effective_spell_power(vitals, runtime),
            language.pick("Summon power", "召唤强度"),
            skills::effective_summon_power(vitals, runtime)
        ));
        lines.push(language.pick("Spell power scales with INT; summon power inherits 70% spell power + 50% hero attack.",
            "法强随智力成长；召唤强度继承70%法强 + 50%英雄攻击力。").into());
        lines.push(
            language
                .pick(
                    "Values include active power bonuses; before enemy armor/resistance.",
                    "数值包含当前强度增幅；尚未扣除敌人护甲或抗性。",
                )
                .into(),
        );
    } else {
        lines.push(format!(
            "{} → {}",
            utility(language, i, rank),
            utility(language, i, next)
        ));
        if i == 29 {
            let mult = skill.definition().rarity.skill_power_multiplier();
            let spell = skills::effective_spell_power(vitals, runtime);
            let summon = skills::effective_summon_power(vitals, runtime);
            let r = f32::from(rank.saturating_sub(1));
            let n = f32::from(next.saturating_sub(1));
            lines.push(format!(
                "{} {:.1} → {:.1}; {} {:.1} → {:.1}",
                language.pick("Lightning / hit", "雷暴每击"),
                (80.0 + 40.0 * r + 0.7 * spell) * mult,
                (80.0 + 40.0 * n + 0.7 * spell) * mult,
                language.pick("fire impact", "火雨命中"),
                (130.0 + 65.0 * r + spell) * mult,
                (130.0 + 65.0 * n + spell) * mult
            ));
            lines.push(format!(
                "{} {:.1} → {:.1}",
                language.pick("Hound damage / hit", "灵犬每击伤害"),
                (45.0 + 18.0 * r + 0.35 * summon) * mult,
                (45.0 + 18.0 * n + 0.35 * summon) * mult
            ));
            lines.push(language.pick("Formulas × 1.85: lightning 80 + 40 × (rank − 1) + 0.70 × spell; fire 130 + 65 × (rank − 1) + spell; hound 45 + 18 × (rank − 1) + 0.35 × summon.",
                "公式均×1.85：雷暴80+40×（等级−1）+0.70法强；火雨130+65×（等级−1）+法强；灵犬45+18×（等级−1）+0.35召唤强度。").into());
        }
    }
    let range_line_index = lines.len();
    if let (Some(a), Some(b)) = (
        skills::skill_effect_radius(i, rank),
        skills::skill_effect_radius(i, next),
    ) {
        lines.push(format!(
            "{} {a:.2} → {b:.2} m",
            language.pick(
                if i == 9 {
                    "Cone reach"
                } else if i == 0 {
                    "Chain hop reach"
                } else if i == 27 {
                    "Pull radius"
                } else {
                    "Effect radius"
                },
                if i == 9 {
                    "锥形距离"
                } else if i == 0 {
                    "弹射距离"
                } else if i == 27 {
                    "牵引半径"
                } else {
                    "效果半径"
                }
            )
        ));
    } else if i == 3 {
        lines.push(format!(
            "{} {:.1} → {:.1} m · {} 24 m",
            language.pick("Wave width", "浪宽"),
            11.0 + f32::from(rank),
            11.0 + f32::from(next),
            language.pick("travel", "行程")
        ));
    } else if i == 30 {
        lines.push(format!(
            "{} {:.1} → {:.1} m · {} {:.2} → {:.2} m",
            language.pick("Reach", "距离"),
            12.0 * skills::rank_size(rank),
            12.0 * skills::rank_size(next),
            language.pick("beam width", "束宽"),
            2.0 * (0.24 + f32::from(rank) * 0.06),
            2.0 * (0.24 + f32::from(next) * 0.06)
        ));
    } else if i == 17 {
        lines.push(format!(
            "{} {:.1} → {:.1} m",
            language.pick("Turret range", "炮塔射程"),
            8.0 * skills::rank_size(rank),
            8.0 * skills::rank_size(next)
        ));
    } else if ![4, 5, 16, 20, 21, 26, 28, 29].contains(&i) {
        lines.push(format!(
            "{} {:.1} m",
            language.pick("Target acquisition range", "索敌距离"),
            build.attack_range + 5.0
        ));
    }
    if lines.len() > range_line_index {
        let range = lines.remove(range_line_index);
        lines.insert(3.min(lines.len()), range);
    }
    if [4, 5, 16, 17, 26, 27].contains(&i) {
        lines.push(format!(
            "{} ×{:.2} → ×{:.2}",
            language.pick("Summon size", "召唤体型"),
            skills::summon_size_multiplier(i, rank),
            skills::summon_size_multiplier(i, next)
        ));
    }
    let count = match i {
        0 => Some((
            skills::arc_chain_hit_count(rank),
            skills::arc_chain_hit_count(next),
        )),
        2 => Some((
            skills::frost_lance_count(rank),
            skills::frost_lance_count(next),
        )),
        4 => Some((skills::bone_sentry_cap(rank), skills::bone_sentry_cap(next))),
        5 => Some((
            skills::spirit_hound_count(rank),
            skills::spirit_hound_count(next),
        )),
        6 => Some((
            skills::orbit_blade_count(rank),
            skills::orbit_blade_count(next),
        )),
        7 => Some((
            skills::piercing_volley_count(rank),
            skills::piercing_volley_count(next),
        )),
        19 => Some((
            skills::flying_sword_count(rank),
            skills::flying_sword_count(next),
        )),
        24 => Some((
            skills::sky_judgment_target_count(rank),
            skills::sky_judgment_target_count(next),
        )),
        _ => None,
    };
    if let Some((a, b)) = count {
        lines.push(format!(
            "{} {a} → {b}",
            language.pick(
                if i == 4 {
                    "Simultaneous guard cap"
                } else {
                    "Hits / projectiles / summons"
                },
                if i == 4 {
                    "同时骨卫上限"
                } else {
                    "命中／弹道／召唤数量"
                }
            )
        ));
    }
    lines.push(simple(language, skill, current));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detailed_preview_uses_live_damage_cooldown_and_next_radius() {
        let build = PlayerBuild::default();
        let vitals = HeroVitals::default();
        let runtime = SkillRuntime::default();
        let detail = detailed(
            GameLanguage::English,
            SkillId(13),
            2,
            &build,
            &vitals,
            &runtime,
        );
        let a = skills::skill_preview_damage(13, 2, &build, &vitals, &runtime).unwrap();
        let b = skills::skill_preview_damage(13, 3, &build, &vitals, &runtime).unwrap();
        assert!(detail.contains(&format!("{a:.1} → {b:.1}")));
        assert!(detail.contains(&format!("Cooldown {:.1}s", skills::rank_cooldown(13, 2))));
        assert!(detail.contains("Effect radius 4.00 → 4.80 m"));
        assert!(detail.contains("spell power"));
        assert!(!simple(GameLanguage::English, SkillId(13), 2).contains("Formula:"));
    }
    #[test]
    fn passive_and_infection_previews_do_not_invent_catalog_damage() {
        let build = PlayerBuild::default();
        let vitals = HeroVitals::default();
        let runtime = SkillRuntime::default();
        let prism = detailed(
            GameLanguage::English,
            SkillId(20),
            4,
            &build,
            &vitals,
            &runtime,
        );
        assert!(prism.contains("Copied spell damage 94% → Copied spell damage 100%"));
        assert!(!prism.contains("Damage ("));
        let infection = detailed(
            GameLanguage::SimplifiedChinese,
            SkillId(18),
            2,
            &build,
            &vitals,
            &runtime,
        );
        assert!(infection.contains("物理疫蚀每跳"));
        assert!(infection.contains("0.14 + 0.04"));
        let volley = detailed(
            GameLanguage::English,
            SkillId(7),
            2,
            &build,
            &vitals,
            &runtime,
        );
        assert!(volley.contains(&format!(
            "{:.1} → {:.1}",
            build.attack_damage * 0.6,
            build.attack_damage * 0.6
        )));
        assert!(volley.contains("Formula: 0.60 × attack damage"));
    }
    #[test]
    fn all_skill_descriptions_are_localized_and_bounded_at_max_rank() {
        let build = PlayerBuild::default();
        let vitals = HeroVitals::default();
        let runtime = SkillRuntime::default();
        for index in 0..SkillId::COUNT {
            for language in [GameLanguage::English, GameLanguage::SimplifiedChinese] {
                let text = detailed(language, SkillId(index as u8), 5, &build, &vitals, &runtime);
                assert!(text.contains("5 → 5/5"));
                assert!(!text.contains("NaN") && !text.contains("infinity"));
                assert!(!text.is_empty());
            }
        }
    }
}
