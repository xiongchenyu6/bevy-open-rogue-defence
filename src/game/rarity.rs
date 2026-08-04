use bevy::prelude::*;

use super::content::Rarity;

/// Shared player-facing palette: R white, SR blue, SSR orange and UR red.
pub(crate) fn rarity_color(rarity: Rarity) -> Color {
    match rarity {
        Rarity::R => Color::srgb(0.90, 0.92, 0.96),
        Rarity::Sr => Color::srgb(0.20, 0.58, 1.0),
        Rarity::Ssr => Color::srgb(1.0, 0.55, 0.10),
        Rarity::Ur => Color::srgb(1.0, 0.14, 0.20),
    }
}
