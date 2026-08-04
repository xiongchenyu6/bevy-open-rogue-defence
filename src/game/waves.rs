//! Deterministic 25-minute wave schedule from `docs/gameplay-spec.md`.

#[derive(Clone, Copy, Debug)]
pub struct WaveDef {
    pub count: u16,
    pub hp_multiplier: f32,
    pub damage_multiplier: f32,
    /// `(enemy catalog index, integer weight)` pairs.
    pub mix: &'static [(usize, u8)],
}

const W01: &[(usize, u8)] = &[(0, 100)];
const W02: &[(usize, u8)] = &[(0, 70), (1, 30)];
const W03: &[(usize, u8)] = &[(0, 55), (1, 25), (3, 20)];
const W04: &[(usize, u8)] = &[(0, 45), (1, 30), (4, 25)];
const W05: &[(usize, u8)] = &[(0, 60), (2, 25), (3, 15)];
const W06: &[(usize, u8)] = &[(0, 40), (1, 35), (3, 25)];
const W07: &[(usize, u8)] = &[(0, 35), (4, 35), (2, 20), (5, 10)];
const W08: &[(usize, u8)] = &[(1, 40), (0, 25), (3, 20), (8, 15)];
const W09: &[(usize, u8)] = &[(0, 35), (3, 25), (2, 25), (5, 15)];
const W10: &[(usize, u8)] = &[(0, 35), (3, 30), (2, 25), (6, 10)];
const W11: &[(usize, u8)] = &[(0, 25), (1, 25), (7, 20), (3, 20), (4, 10)];
const W12: &[(usize, u8)] = &[(1, 30), (9, 20), (0, 20), (3, 15), (8, 15)];
const W13: &[(usize, u8)] = &[(0, 25), (2, 20), (7, 20), (4, 20), (5, 15)];
const W14: &[(usize, u8)] = &[(0, 30), (3, 25), (9, 20), (8, 15), (6, 10)];
const W15: &[(usize, u8)] = &[(2, 25), (0, 25), (4, 20), (6, 15), (10, 15)];
const W16: &[(usize, u8)] = &[(1, 25), (7, 20), (9, 20), (3, 20), (5, 15)];
const W17: &[(usize, u8)] = &[(4, 25), (2, 20), (0, 20), (3, 20), (8, 15)];
const W18: &[(usize, u8)] = &[(7, 20), (9, 20), (0, 20), (3, 15), (10, 15), (6, 10)];
const W19: &[(usize, u8)] = &[(0, 20), (2, 20), (4, 20), (5, 15), (8, 15), (11, 10)];
const W20: &[(usize, u8)] = &[(0, 20), (1, 20), (2, 20), (6, 15), (10, 15), (11, 10)];
const W21: &[(usize, u8)] = &[(7, 20), (4, 20), (3, 20), (10, 15), (0, 15), (9, 10)];
const W22: &[(usize, u8)] = &[(2, 20), (3, 20), (0, 15), (5, 15), (8, 15), (6, 15)];
const W23: &[(usize, u8)] = &[(4, 20), (9, 20), (0, 15), (7, 15), (10, 15), (11, 15)];
const W24: &[(usize, u8)] = &[(2, 20), (1, 20), (6, 15), (11, 15), (5, 15), (8, 15)];
const W25: &[(usize, u8)] = &[(0, 25), (6, 15), (11, 15), (10, 15), (9, 15), (7, 15)];

pub const WAVES: [WaveDef; 25] = [
    WaveDef {
        count: 18,
        hp_multiplier: 1.00,
        damage_multiplier: 1.00,
        mix: W01,
    },
    WaveDef {
        count: 24,
        hp_multiplier: 1.14,
        damage_multiplier: 1.06,
        mix: W02,
    },
    WaveDef {
        count: 28,
        hp_multiplier: 1.30,
        damage_multiplier: 1.12,
        mix: W03,
    },
    WaveDef {
        count: 32,
        hp_multiplier: 1.48,
        damage_multiplier: 1.20,
        mix: W04,
    },
    WaveDef {
        count: 24,
        hp_multiplier: 1.70,
        damage_multiplier: 1.30,
        mix: W05,
    },
    WaveDef {
        count: 40,
        hp_multiplier: 1.98,
        damage_multiplier: 1.42,
        mix: W06,
    },
    WaveDef {
        count: 44,
        hp_multiplier: 2.30,
        damage_multiplier: 1.55,
        mix: W07,
    },
    WaveDef {
        count: 48,
        hp_multiplier: 2.67,
        damage_multiplier: 1.70,
        mix: W08,
    },
    WaveDef {
        count: 52,
        hp_multiplier: 3.10,
        damage_multiplier: 1.86,
        mix: W09,
    },
    WaveDef {
        count: 42,
        hp_multiplier: 3.60,
        damage_multiplier: 2.05,
        mix: W10,
    },
    WaveDef {
        count: 58,
        hp_multiplier: 4.18,
        damage_multiplier: 2.27,
        mix: W11,
    },
    WaveDef {
        count: 64,
        hp_multiplier: 4.85,
        damage_multiplier: 2.51,
        mix: W12,
    },
    WaveDef {
        count: 68,
        hp_multiplier: 5.63,
        damage_multiplier: 2.78,
        mix: W13,
    },
    WaveDef {
        count: 72,
        hp_multiplier: 6.54,
        damage_multiplier: 3.08,
        mix: W14,
    },
    WaveDef {
        count: 56,
        hp_multiplier: 7.59,
        damage_multiplier: 3.42,
        mix: W15,
    },
    WaveDef {
        count: 78,
        hp_multiplier: 8.81,
        damage_multiplier: 3.80,
        mix: W16,
    },
    WaveDef {
        count: 84,
        hp_multiplier: 10.22,
        damage_multiplier: 4.23,
        mix: W17,
    },
    WaveDef {
        count: 90,
        hp_multiplier: 11.86,
        damage_multiplier: 4.71,
        mix: W18,
    },
    WaveDef {
        count: 96,
        hp_multiplier: 13.76,
        damage_multiplier: 5.25,
        mix: W19,
    },
    WaveDef {
        count: 72,
        hp_multiplier: 15.97,
        damage_multiplier: 5.85,
        mix: W20,
    },
    WaveDef {
        count: 104,
        hp_multiplier: 18.53,
        damage_multiplier: 6.52,
        mix: W21,
    },
    WaveDef {
        count: 112,
        hp_multiplier: 21.50,
        damage_multiplier: 7.26,
        mix: W22,
    },
    WaveDef {
        count: 120,
        hp_multiplier: 24.94,
        damage_multiplier: 8.08,
        mix: W23,
    },
    WaveDef {
        count: 128,
        hp_multiplier: 28.93,
        damage_multiplier: 9.00,
        mix: W24,
    },
    WaveDef {
        count: 96,
        hp_multiplier: 33.56,
        damage_multiplier: 10.00,
        mix: W25,
    },
];

pub fn choose_archetype(wave: WaveDef, roll: u32) -> usize {
    let total: u32 = wave.mix.iter().map(|(_, weight)| u32::from(*weight)).sum();
    let mut cursor = roll % total.max(1);
    for (archetype, weight) in wave.mix {
        let weight = u32::from(*weight);
        if cursor < weight {
            return *archetype;
        }
        cursor -= weight;
    }
    wave.mix.last().map(|entry| entry.0).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_has_exact_duration_and_valid_mixes() {
        assert_eq!(WAVES.len(), 25);
        assert_eq!(
            WAVES.iter().map(|wave| wave.count as u32).sum::<u32>(),
            1_650
        );
        for wave in WAVES {
            assert_eq!(
                wave.mix.iter().map(|entry| entry.1 as u16).sum::<u16>(),
                100
            );
            assert!(wave.mix.iter().all(|entry| entry.0 < 12));
        }
    }

    #[test]
    fn weighted_choice_is_deterministic_and_in_range() {
        for (wave_index, wave) in WAVES.into_iter().enumerate() {
            for roll in 0..256 {
                let choice = choose_archetype(wave, roll);
                assert!(
                    wave.mix.iter().any(|entry| entry.0 == choice),
                    "wave {wave_index}"
                );
            }
        }
    }
}
