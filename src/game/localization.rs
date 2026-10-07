use bevy::prelude::*;

use super::content::{BOSSES, HEROES};
use super::hero_abilities::ultimate_profile;
use super::{BondCardId, BondId, HeroKind, SkillId};

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
pub(super) struct UiFont(pub(super) Handle<Font>);

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

fn apply_ui_font(
    added: On<Add, TextFont>,
    ui_font: Res<UiFont>,
    mut fonts: Query<&mut TextFont, Without<super::asset_readiness::LoadingFallback>>,
) {
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
        match skill.0 {
            0 => "VIOLET CHAIN",
            1 => "FIRE ARROW",
            2 => "FROST ARROW",
            3 => "BOUNDLESS SEA",
            4 => "RAISE DEAD",
            6 => "RETURNING WIND BLADE",
            7 => "ARCHERY",
            12 => "THUNDER STORM",
            14 => "FALLING ICE",
            15 => "TORNADO",
            16 => "WATER ELEMENTAL",
            26 => "FROST BONE DRAGON",
            30 => "FLAME RAY",
            31 => "GOBLIN MINE",
            _ => return skill.title(),
        }
        .to_string()
    }
}

pub fn bond_name(language: GameLanguage, bond: BondId) -> String {
    if language.is_chinese() {
        bond.definition().name.to_string()
    } else {
        bond.definition().name_en.to_string()
    }
}

pub fn bond_card_name(language: GameLanguage, card: BondCardId) -> String {
    let def = card.definition();
    language.pick(def.name_en, def.name).to_string()
}

pub fn bond_family_detail(
    language: GameLanguage,
    family: BondId,
    build: &super::PlayerBuild,
) -> String {
    let def = family.definition();
    let members = def
        .members
        .iter()
        .map(|card| {
            format!(
                "{} {}",
                if build.owned_bond_cards[card.index()] {
                    "✓"
                } else {
                    "○"
                },
                bond_card_name(language, *card)
            )
        })
        .collect::<Vec<_>>()
        .join(" · ");
    let next_bonus = def
        .bonuses
        .iter()
        .find(|bonus| bonus.required > build.bond(family));
    let current_bonus = def
        .bonuses
        .iter()
        .rev()
        .find(|bonus| bonus.required <= build.bond(family));
    let bonuses = next_bonus
        .or(current_bonus)
        .into_iter()
        .map(|bonus| {
            format!(
                "{}: {}",
                bonus.required,
                language.pick(bonus.description_en, bonus.description)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let unlocks = super::BondId::ALL
        .iter()
        .filter(|id| id.definition().prerequisite == Some(family))
        .map(|id| bond_name(language, *id))
        .collect::<Vec<_>>()
        .join(", ");
    let mut result = format!(
        "{} · {}/{}\n{}",
        bond_name(language, family),
        build.bond(family),
        family.required(),
        members
    );
    if !bonuses.is_empty() {
        result.push_str(&format!("\n{bonuses}"));
    }
    if build.bond_complete(family) {
        result = format!(
            "{} · {}\n{}\n{}",
            bond_name(language, family),
            language.pick("COMPLETE", "已集齐"),
            language.pick("Effects active · Slot released", "效果生效 · 已释放羁绊槽"),
            bonuses,
        );
    }
    if !unlocks.is_empty() {
        result.push_str(&format!(
            "\n{}: {unlocks}",
            if build.bond_complete(family) {
                language.pick("Pools unlocked", "已解锁卡池")
            } else {
                language.pick("Completion unlocks", "集齐解锁")
            }
        ));
    }
    result
}

pub fn boss_name(language: GameLanguage, index: usize) -> String {
    let boss = &BOSSES[index.min(BOSSES.len() - 1)];
    language.pick(boss.name_en, boss.name).to_string()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    const UI_FONT_BYTES: &[u8] = include_bytes!("../../assets/fonts/rogue_commander_ui.ttf");
    const PLAYER_FACING_SOURCES: &[&str] = &[
        include_str!("asset_readiness.rs"),
        include_str!("onboarding.rs"),
        include_str!("content.rs"),
        include_str!("bond_catalog.rs"),
        include_str!("bond_catalog/expanded.rs"),
        include_str!("skill_evolution.rs"),
        include_str!("ur_evolution.rs"),
        include_str!("campaign_ui.rs"),
        include_str!("survival_ui.rs"),
        include_str!("hero.rs"),
        include_str!("hero_draft.rs"),
        include_str!("localization.rs"),
        include_str!("market.rs"),
        include_str!("progression.rs"),
        include_str!("progression_ui.rs"),
        include_str!("ui.rs"),
        include_str!("boss_status.rs"),
        include_str!("ui_art.rs"),
        include_str!("bond_collection_ui.rs"),
        include_str!("hero_stats_ui.rs"),
        include_str!("basic_attack_details.rs"),
        include_str!("basic_relics.rs"),
        include_str!("encounter_bosses.rs"),
        include_str!("encounter_boss_ui.rs"),
        include_str!("equipment_ui.rs"),
        include_str!("equipment_ui/panel.rs"),
        include_str!("crafting.rs"),
        include_str!("hero_fusion_ui.rs"),
        include_str!("skill_feedback.rs"),
        include_str!("bond_feedback.rs"),
        include_str!("settings.rs"),
        include_str!("settings/audio_controls.rs"),
        include_str!("settings/run_actions.rs"),
        include_str!("settings/display_controls.rs"),
        include_str!("selection_detail.rs"),
        include_str!("skill_details.rs"),
        include_str!("debug_menu.rs"),
        include_str!("encounters.rs"),
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
        assert_eq!(
            skill_name(GameLanguage::English, SkillId(0)),
            "VIOLET CHAIN"
        );
        assert_eq!(
            bond_name(GameLanguage::SimplifiedChinese, BondId::Journey),
            "启程"
        );
    }

    #[test]
    fn gold_challenge_has_a_single_chinese_name_in_player_facing_text() {
        let retired_name = concat!("金币", "试炼");
        for source in PLAYER_FACING_SOURCES {
            assert!(!source.contains(retired_name));
        }
        assert!(include_str!("ui.rs").contains("金币挑战"));
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
