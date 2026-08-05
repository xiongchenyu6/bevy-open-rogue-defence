use bevy::prelude::*;

use super::combat::BOSS_NAMES;
use super::content::{Attribute, BOSSES, HEROES, SkillEffectKind};
use super::hero_abilities::ultimate_profile;
use super::{BondId, HeroKind, SkillId};

/// One bundled face is used for every Bevy text node so Web builds never rely
/// on unavailable system-font fallback for CJK glyphs.
pub const UI_FONT_PATH: &str = "fonts/rogue_commander_ui.ttf";

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GameLanguage {
    #[default]
    English,
    SimplifiedChinese,
}

impl GameLanguage {
    pub const ALL: [Self; 2] = [Self::English, Self::SimplifiedChinese];

    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace('_', "-");
        match normalized.as_str() {
            "en" | "en-us" | "en-gb" | "english" => Some(Self::English),
            "zh" | "zh-cn" | "zh-hans" | "cn" | "chinese" => Some(Self::SimplifiedChinese),
            _ => None,
        }
    }

    pub const fn query_code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-CN",
        }
    }

    pub const fn html_lang(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::SimplifiedChinese => "zh-Hans",
        }
    }

    pub const fn is_chinese(self) -> bool {
        matches!(self, Self::SimplifiedChinese)
    }

    pub const fn pick<'a>(self, english: &'a str, chinese: &'a str) -> &'a str {
        match self {
            Self::English => english,
            Self::SimplifiedChinese => chinese,
        }
    }
}

#[derive(Resource, Clone)]
struct UiFont(Handle<Font>);

impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self(asset_server.load(UI_FONT_PATH))
    }
}

pub struct LocalizationPlugin;

impl Plugin for LocalizationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameLanguage>()
            .init_resource::<UiFont>()
            .add_observer(apply_ui_font);
    }
}

fn apply_ui_font(added: On<Add, TextFont>, ui_font: Res<UiFont>, mut fonts: Query<&mut TextFont>) {
    if let Ok(mut font) = fonts.get_mut(added.entity) {
        font.font = ui_font.0.clone().into();
    }
}

pub fn hero_name(language: GameLanguage, hero: HeroKind) -> String {
    if language.is_chinese() {
        HEROES[hero.index()].name.to_string()
    } else {
        HEROES[hero.index()]
            .id
            .replace('_', " ")
            .to_ascii_uppercase()
    }
}

pub fn hero_ultimate_name(language: GameLanguage, hero: HeroKind) -> String {
    if language.is_chinese() {
        HEROES[hero.index()].ultimate.name.to_string()
    } else {
        ultimate_profile(hero).name.to_string()
    }
}

pub fn skill_name(language: GameLanguage, skill: SkillId) -> String {
    if language.is_chinese() {
        skill.definition().name.to_string()
    } else {
        skill.title()
    }
}

pub fn bond_name(language: GameLanguage, bond: BondId) -> String {
    if language.is_chinese() {
        bond.definition().name.to_string()
    } else {
        bond.title().to_string()
    }
}

pub fn skill_card_description(language: GameLanguage, skill: SkillId) -> String {
    let def = skill.definition();
    let rarity = def.rarity.label();
    let multiplier = def.rarity.skill_power_multiplier();
    let cooldown = def.cooldown_secs * def.rarity.skill_cooldown_multiplier();
    let kind = skill_kind_name(language, def.effect.kind);
    if language.is_chinese() {
        if def.cooldown_secs > 0.0 {
            format!(
                "[{rarity}] {kind} · {cooldown:.1}秒\n威力 ×{multiplier:.2} · {:.0} + {:.0}/级",
                def.effect.power.base, def.effect.power.per_rank
            )
        } else {
            format!(
                "[{rarity}] {kind}\n威力 ×{multiplier:.2} · {:.0} + {:.0}/级",
                def.effect.power.base, def.effect.power.per_rank
            )
        }
    } else {
        skill.description()
    }
}

pub fn bond_card_description(language: GameLanguage, bond: BondId) -> String {
    if !language.is_chinese() {
        return format!(
            "[{}] +{} BOND POINTS\n{}",
            bond.rarity().label(),
            bond.rarity().bond_points_per_pick(),
            bond.description()
        );
    }

    let summary = match bond {
        BondId::Thunder => "2/4/6：攻速、连锁、裁决",
        BondId::Ember => "2/4/6：灼烧、暴击灼烧、引爆",
        BondId::Spirits => "2/4/6：召唤上限、标记、回响",
        BondId::Elements => "2/4/6：元素反应、威力、回响",
        BondId::Arcane => "2/4/6：冷却缩减与法术回响",
        BondId::Triune => "2/4/6：攻击、法术、召唤融合",
        BondId::Ascension => "2/4/6：飞剑与登临",
        BondId::Blood => "2/4/6：暴击、处决、狂热",
        BondId::Fortune => "2/4/6：收入、品阶、幸运抽取",
        BondId::Bastion => "2/4/6：城门耐久、拦截、背水一战",
    };
    format!(
        "[{}] +{} 羁绊点\n{summary}",
        bond.rarity().label(),
        bond.rarity().bond_points_per_pick()
    )
}

pub fn boss_name(language: GameLanguage, index: usize) -> String {
    let index = index.min(BOSSES.len() - 1);
    if language.is_chinese() {
        BOSSES[index].name.to_string()
    } else {
        BOSS_NAMES[index].to_string()
    }
}

pub const fn attribute_name(language: GameLanguage, attribute: Attribute) -> &'static str {
    match (language, attribute) {
        (GameLanguage::English, Attribute::Strength) => "STR",
        (GameLanguage::English, Attribute::Agility) => "AGI",
        (GameLanguage::English, Attribute::Intelligence) => "INT",
        (GameLanguage::SimplifiedChinese, Attribute::Strength) => "力量",
        (GameLanguage::SimplifiedChinese, Attribute::Agility) => "敏捷",
        (GameLanguage::SimplifiedChinese, Attribute::Intelligence) => "智力",
    }
}

pub const fn skill_kind_name(language: GameLanguage, kind: SkillEffectKind) -> &'static str {
    match (language, kind) {
        (GameLanguage::English, SkillEffectKind::Projectile) => "PROJECTILE",
        (GameLanguage::English, SkillEffectKind::Nova) => "NOVA",
        (GameLanguage::English, SkillEffectKind::GroundZone) => "GROUND ZONE",
        (GameLanguage::English, SkillEffectKind::AttackProc) => "ATTACK PROC",
        (GameLanguage::English, SkillEffectKind::Orbit) => "ORBIT",
        (GameLanguage::English, SkillEffectKind::Summon) => "SUMMON",
        (GameLanguage::English, SkillEffectKind::Mark) => "MARK",
        (GameLanguage::English, SkillEffectKind::Echo) => "ECHO",
        (GameLanguage::English, SkillEffectKind::Revive) => "REVIVE",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Projectile) => "投射物",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Nova) => "新星",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::GroundZone) => "区域",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::AttackProc) => "攻击触发",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Orbit) => "环绕",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Summon) => "召唤",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Mark) => "标记",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Echo) => "回响",
        (GameLanguage::SimplifiedChinese, SkillEffectKind::Revive) => "复活",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const UI_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/rogue_commander_ui.ttf");
    const PLAYER_FACING_SOURCES: &[&str] = &[
        include_str!("content.rs"),
        include_str!("hero.rs"),
        include_str!("localization.rs"),
        include_str!("market.rs"),
        include_str!("progression.rs"),
        include_str!("ui.rs"),
        include_str!("../../web/index.html"),
    ];

    #[test]
    fn language_parser_accepts_canonical_and_browser_style_codes() {
        assert_eq!(GameLanguage::parse("en"), Some(GameLanguage::English));
        assert_eq!(
            GameLanguage::parse("zh_Hans"),
            Some(GameLanguage::SimplifiedChinese)
        );
        assert_eq!(
            GameLanguage::parse("ZH-cn"),
            Some(GameLanguage::SimplifiedChinese)
        );
        assert_eq!(GameLanguage::parse("fr"), None);
    }

    #[test]
    fn catalog_names_are_localized_without_changing_ids() {
        assert_eq!(
            hero_name(GameLanguage::SimplifiedChinese, HeroKind::StormSavant),
            "玄霆术师"
        );
        assert_eq!(skill_name(GameLanguage::English, SkillId(0)), "ARC CHAIN");
        assert_eq!(
            bond_name(GameLanguage::SimplifiedChinese, BondId::Thunder),
            "天雷导体"
        );
    }

    #[test]
    fn bundled_ui_font_covers_every_player_facing_character() {
        assert!(
            UI_FONT_BYTES.len() < 2_000_000,
            "UI font subset unexpectedly grew to {} bytes",
            UI_FONT_BYTES.len()
        );
        let face = ttf_parser::Face::parse(UI_FONT_BYTES, 0)
            .expect("bundled UI font should be a valid TrueType face");
        let mut required: BTreeSet<char> = (' '..='~').collect();
        required.extend(
            PLAYER_FACING_SOURCES
                .iter()
                .flat_map(|source| source.chars())
                .filter(|character| !character.is_control()),
        );
        let missing: Vec<String> = required
            .into_iter()
            .filter(|character| face.glyph_index(*character).is_none())
            .map(|character| format!("{character} (U+{:04X})", character as u32))
            .collect();
        assert!(
            missing.is_empty(),
            "bundled UI font is missing: {}",
            missing.join(", ")
        );
    }
}
