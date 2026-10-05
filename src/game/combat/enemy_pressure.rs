//! Ordinary road enemies gain pressure without changing the finite wave budget.
use bevy::prelude::Resource;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnemyPressure {
    pub health: f32,
    pub movement: f32,
    pub attack: f32,
}
impl EnemyPressure {
    pub const BASELINE: Self = Self {
        health: 1.0,
        movement: 1.0,
        attack: 1.0,
    };
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct EnemyPressureSettings {
    pub health_growth: f32,
    pub health_cap: f32,
    pub movement_growth: f32,
    pub attack_growth: f32,
    pub movement_cap: f32,
    pub attack_cap: f32,
}
impl Default for EnemyPressureSettings {
    fn default() -> Self {
        Self {
            health_growth: 0.12,
            health_cap: 3.7,
            movement_growth: 0.03,
            attack_growth: 0.05,
            movement_cap: 1.8,
            attack_cap: 2.2,
        }
    }
}
impl EnemyPressureSettings {
    pub fn at_wave(&self, wave: u32) -> EnemyPressure {
        let growth_steps = wave.saturating_sub(3).min(22) as f32;
        EnemyPressure {
            health: (1.0 + growth_steps * self.health_growth.max(0.0))
                .min(self.health_cap.max(1.0)),
            movement: (1.0 + growth_steps * self.movement_growth.max(0.0))
                .min(self.movement_cap.max(1.0)),
            attack: (1.0 + growth_steps * self.attack_growth.max(0.0))
                .min(self.attack_cap.max(1.0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_starts_after_wave_three_and_is_bounded() {
        let settings = EnemyPressureSettings::default();
        for wave in 0..=3 {
            assert_eq!(settings.at_wave(wave), EnemyPressure::BASELINE);
        }
        assert!((settings.at_wave(10).health - 1.84).abs() < 0.001);
        assert!((settings.at_wave(25).health - 3.64).abs() < 0.001);
        assert!((settings.at_wave(10).movement - 1.21).abs() < 0.001);
        assert!((settings.at_wave(10).attack - 1.35).abs() < 0.001);
        assert!((settings.at_wave(25).movement - 1.66).abs() < 0.001);
        assert!((settings.at_wave(25).attack - 2.1).abs() < 0.001);
        assert_eq!(settings.at_wave(u32::MAX), settings.at_wave(25));
        let override_settings = EnemyPressureSettings {
            health_growth: 1.0,
            movement_growth: 1.0,
            attack_growth: 2.0,
            ..settings
        };
        assert_eq!(
            override_settings.at_wave(25),
            EnemyPressure {
                health: 3.7,
                movement: 1.8,
                attack: 2.2
            }
        );
    }
}
