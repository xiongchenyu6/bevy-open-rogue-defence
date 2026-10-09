//! Six one-time recipes retain ingredient effects and consume inventory entries.
use super::market::{MarketEffects, MarketItem};
use super::{GameLanguage, HeroVitals};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecipeId {
    Bloodreaver,
    AstralPrism,
    StormBastion,
    Suncoil,
    NightCrown,
    WorldEngine,
}
impl RecipeId {
    pub const ALL: [Self; 6] = [
        Self::Bloodreaver,
        Self::AstralPrism,
        Self::StormBastion,
        Self::Suncoil,
        Self::NightCrown,
        Self::WorldEngine,
    ];
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn name(self, language: GameLanguage) -> &'static str {
        let (en, zh) = match self {
            Self::Bloodreaver => ("Bloodreaver", "血饮战刃"),
            Self::AstralPrism => ("Astral Prism", "星界棱镜"),
            Self::StormBastion => ("Storm Bastion", "风暴壁垒"),
            Self::Suncoil => ("Suncoil", "日耀导体"),
            Self::NightCrown => ("Night Crown", "永夜王冠"),
            Self::WorldEngine => ("World Engine", "世界引擎"),
        };
        language.pick(en, zh)
    }
    pub(crate) fn icon(self) -> &'static str {
        match self {
            Self::Bloodreaver => MarketItem::SerratedDisc.icon_path(),
            Self::AstralPrism => MarketItem::ArcLens.icon_path(),
            Self::StormBastion => MarketItem::HardenedPlate.icon_path(),
            Self::Suncoil => MarketItem::StormCoil.icon_path(),
            Self::NightCrown => MarketItem::AscendantCrown.icon_path(),
            Self::WorldEngine => MarketItem::PrismaticEngine.icon_path(),
        }
    }
    pub(crate) fn rarity(self) -> super::content::Rarity {
        if self.index() >= 4 {
            super::content::Rarity::Ur
        } else {
            super::content::Rarity::Ssr
        }
    }
    pub fn bonus(self, language: GameLanguage) -> &'static str {
        let (en, zh) = match self {
            Self::Bloodreaver => (
                "Extra: +3 armor penetration, +5% lifesteal",
                "额外：物理穿透+3点，吸血+5%",
            ),
            Self::AstralPrism => (
                "Extra: +10% magic penetration, +15% spell power",
                "额外：法术穿透+10%，法术威力+15%",
            ),
            Self::StormBastion => ("Extra: +4 armor, +5% evasion", "额外：护甲+4，闪避+5%"),
            Self::Suncoil => (
                "Extra: +5% critical chance, +15% spell power",
                "额外：暴击率+5%，法术威力+15%",
            ),
            Self::NightCrown => (
                "Extra: +4 armor penetration, +5% lifesteal, +4 armor",
                "额外：物理穿透+4点，吸血+5%，护甲+4",
            ),
            Self::WorldEngine => (
                "Extra: +10% magic penetration, +10% skill echo chance",
                "额外：法术穿透+10%，技能回响概率+10%",
            ),
        };
        language.pick(en, zh)
    }
    pub fn full_effect(self, language: GameLanguage) -> String {
        let mut effects = Vec::new();
        for item in self.ingredients() {
            match item {
                Ingredient::Gear(item) => effects.push(format!(
                    "{}: {}",
                    item.name(language),
                    item.effect_text(language)
                )),
                Ingredient::Crafted(recipe) => effects.push(format!(
                    "{}\n{}",
                    recipe.name(language),
                    recipe.full_effect(language)
                )),
            }
        }
        format!(
            "{}\n{}\n{}",
            self.bonus(language),
            language.pick("All ingredient effects inherited:", "继承全部材料效果："),
            effects.join("\n")
        )
    }

    fn ingredients(self) -> &'static [Ingredient] {
        use Ingredient::{Crafted, Gear};
        match self {
            Self::Bloodreaver => &[
                Gear(MarketItem::SerratedDisc),
                Gear(MarketItem::BloodContract),
            ],
            Self::AstralPrism => &[Gear(MarketItem::ArcLens), Gear(MarketItem::EchoCore)],
            Self::StormBastion => &[Gear(MarketItem::HardenedPlate), Gear(MarketItem::WindBoots)],
            Self::Suncoil => &[Gear(MarketItem::EmberHeart), Gear(MarketItem::StormCoil)],
            Self::NightCrown => &[
                Crafted(Self::Bloodreaver),
                Crafted(Self::StormBastion),
                Gear(MarketItem::AscendantCrown),
            ],
            Self::WorldEngine => &[
                Crafted(Self::AstralPrism),
                Crafted(Self::Suncoil),
                Gear(MarketItem::PrismaticEngine),
            ],
        }
    }
    pub fn materials(self, language: GameLanguage) -> String {
        self.ingredients()
            .iter()
            .map(|item| item.name(language))
            .collect::<Vec<_>>()
            .join(" + ")
    }
}
#[derive(Clone, Copy)]
enum Ingredient {
    Gear(MarketItem),
    Crafted(RecipeId),
}
impl Ingredient {
    fn name(self, language: GameLanguage) -> &'static str {
        match self {
            Self::Gear(item) => item.name(language),
            Self::Crafted(recipe) => recipe.name(language),
        }
    }
}
#[derive(Resource, Default)]
pub struct CraftingState {
    active: u8,
    acquired: u8,
    pub(crate) revision: u32,
}
impl CraftingState {
    pub fn owns(&self, recipe: RecipeId) -> bool {
        self.active & (1 << recipe.index()) != 0
    }
    pub fn acquired(&self, recipe: RecipeId) -> bool {
        self.acquired & (1 << recipe.index()) != 0
    }
    pub fn equipment(&self) -> Vec<RecipeId> {
        RecipeId::ALL
            .into_iter()
            .filter(|recipe| self.owns(*recipe))
            .collect()
    }
    pub(crate) fn ready(&self, recipe: RecipeId, market: &MarketEffects) -> bool {
        !self.acquired(recipe)
            && recipe.ingredients().iter().all(|item| match item {
                Ingredient::Gear(item) => market.equipment().contains(item),
                Ingredient::Crafted(recipe) => self.owns(*recipe),
            })
    }
    pub(crate) fn material_status(
        &self,
        recipe: RecipeId,
        market: &MarketEffects,
        language: GameLanguage,
    ) -> String {
        recipe
            .ingredients()
            .iter()
            .map(|item| {
                let owned = match item {
                    Ingredient::Gear(item) => market.equipment().contains(item),
                    Ingredient::Crafted(recipe) => self.owns(*recipe),
                };
                format!("{} {}", if owned { "✓" } else { "○" }, item.name(language))
            })
            .collect::<Vec<_>>()
            .join(" + ")
    }
    fn craft(
        &mut self,
        recipe: RecipeId,
        market: &mut MarketEffects,
        vitals: &mut HeroVitals,
    ) -> bool {
        if !self.ready(recipe, market) {
            return false;
        }
        for item in recipe.ingredients() {
            match item {
                Ingredient::Gear(item) => market.consume_equipment(*item),
                Ingredient::Crafted(recipe) => self.active &= !(1 << recipe.index()),
            }
        }
        self.active |= 1 << recipe.index();
        self.acquired |= 1 << recipe.index();
        market.apply_crafting_bonus(recipe, vitals);
        self.revision += 1;
        true
    }
}
pub fn craft_recipe(world: &mut World, recipe: RecipeId) -> bool {
    world.resource_scope(|world, mut state: Mut<CraftingState>| {
        world.resource_scope(|world, mut market: Mut<MarketEffects>| {
            state.craft(recipe, &mut market, &mut world.resource_mut::<HeroVitals>())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn both_advanced_chains_require_and_consume_their_real_predecessors_once() {
        let (_, mut vitals, mut market) =
            super::super::market::purchased_crafting_materials_for_test();
        let mut state = CraftingState::default();
        for recipe in [RecipeId::NightCrown, RecipeId::WorldEngine] {
            assert!(!state.craft(recipe, &mut market, &mut vitals));
        }
        for recipe in [
            RecipeId::Bloodreaver,
            RecipeId::StormBastion,
            RecipeId::NightCrown,
            RecipeId::AstralPrism,
            RecipeId::Suncoil,
            RecipeId::WorldEngine,
        ] {
            assert!(state.craft(recipe, &mut market, &mut vitals));
            assert!(!state.craft(recipe, &mut market, &mut vitals));
        }
        assert_eq!(
            state.equipment(),
            vec![RecipeId::NightCrown, RecipeId::WorldEngine]
        );
        assert!(market.equipment().is_empty());
        assert_eq!(market.physical_penetration(), 10.0);
        assert!((market.spell_penetration() - 0.30).abs() < 0.0001);
        assert!((market.life_leech() - 0.15).abs() < 0.0001);
        assert_eq!(vitals.armor, 14.0);
        assert!((market.spell_echo_chance() - 0.30).abs() < 0.0001);
        assert_eq!(state.revision, 6);
    }

    #[test]
    fn crafting_consumes_inventory_once_retains_all_bonuses_and_checks_advanced_materials() {
        let (_, mut vitals, mut market) =
            super::super::market::purchased_attribute_equipment_for_test();
        let mut state = CraftingState::default();
        assert!(!state.craft(RecipeId::NightCrown, &mut market, &mut vitals));
        assert!(state.craft(RecipeId::Bloodreaver, &mut market, &mut vitals));
        assert_eq!(market.physical_penetration(), 6.0);
        assert!((market.life_leech() - 0.10).abs() < 0.0001);
        assert!(!market.equipment().contains(&MarketItem::SerratedDisc));
        assert!(!market.equipment().contains(&MarketItem::BloodContract));
        assert!(!state.craft(RecipeId::Bloodreaver, &mut market, &mut vitals));
        assert_eq!(market.physical_penetration(), 6.0);
        assert!(state.craft(RecipeId::StormBastion, &mut market, &mut vitals));
        assert_eq!(vitals.armor, 10.0);
        assert!((market.evasion() - 0.13).abs() < 0.0001);
        assert!(!state.craft(RecipeId::NightCrown, &mut market, &mut vitals));
        assert!(state.owns(RecipeId::Bloodreaver));
        assert!(state.owns(RecipeId::StormBastion));
    }
}
