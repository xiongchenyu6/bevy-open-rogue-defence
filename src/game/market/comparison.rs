//! Preview the real transaction on disposable state, never a second balance table.
use super::*;

pub(super) fn preview(
    item: MarketItem,
    language: GameLanguage,
    build: &PlayerBuild,
    vitals: &HeroVitals,
    effects: &MarketEffects,
) -> String {
    if item.is_consumable() || effects.owns(item) {
        return String::new();
    }
    let mut next_build = build.clone();
    let mut next_vitals = vitals.clone();
    let mut next_effects = effects.clone();
    item.apply(
        &mut GameSession::default(),
        &mut next_build,
        &mut next_vitals,
        &mut next_effects,
    );
    let before = values(build, vitals, effects);
    let after = values(&next_build, &next_vitals, &next_effects);
    let labels = [
        ("Base attack", "基础攻击"),
        ("Max health", "生命上限"),
        ("Armor", "护甲"),
        ("Spell power", "法术强度"),
        ("Move speed", "移动速度"),
        ("Armor penetration", "物理穿透"),
        ("Magic penetration", "法术穿透"),
        ("Equipment evasion", "装备闪避"),
        ("Equipment critical", "装备暴击"),
        ("Lifesteal", "吸血"),
        ("Spell echo", "技能回响"),
        ("Summon attack speed", "召唤攻速"),
        ("Reaction damage", "反应伤害"),
        ("Summon power", "召唤强度"),
        ("Extra summons", "额外召唤"),
    ];
    before
        .iter()
        .zip(after)
        .enumerate()
        .filter(|(_, (a, b))| (**a - *b).abs() > 0.0001)
        .take(3)
        .map(|(i, (a, b))| {
            let suffix = if matches!(i, 6..=12) { "%" } else { "" };
            format!(
                "{}  {a:.1}{suffix} → {b:.1}{suffix}",
                language.pick(labels[i].0, labels[i].1)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn values(build: &PlayerBuild, vitals: &HeroVitals, effects: &MarketEffects) -> [f32; 15] {
    [
        build.attack_damage,
        vitals.max_health,
        vitals.armor,
        effects.effective_spell_power(vitals.spell_power()),
        build.move_speed,
        effects.physical_penetration(),
        effects.spell_penetration() * 100.,
        effects.evasion() * 100.,
        effects.critical_chance() * 100.,
        effects.life_leech() * 100.,
        effects.spell_echo_chance() * 100.,
        effects.summon_attack_speed_multiplier() * 100.,
        effects.element_reaction_multiplier() * 100.,
        effects.effective_summon_power(vitals.summon_power()),
        f32::from(effects.summon_cap_bonus()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shopping_comparison_uses_transaction_without_changing_live_state() {
        let build = PlayerBuild::default();
        let vitals = HeroVitals::for_hero(super::super::super::HeroKind::StoneWarden, 1);
        let effects = MarketEffects::default();
        let copy = preview(
            MarketItem::HardenedPlate,
            GameLanguage::English,
            &build,
            &vitals,
            &effects,
        );
        assert!(copy.contains(&format!(
            "{:.1} → {:.1}",
            vitals.max_health,
            vitals.max_health * 1.2
        )));
        assert!(copy.contains("Armor  0.0 → 6.0"));
        assert_eq!(effects.equipment().len(), 0);
        assert_eq!(vitals.armor, 0.);
        let (_, _, owned) = super::super::purchased_attribute_equipment_for_test();
        assert!(
            preview(
                MarketItem::HardenedPlate,
                GameLanguage::English,
                &build,
                &vitals,
                &owned
            )
            .is_empty()
        );
    }
}
