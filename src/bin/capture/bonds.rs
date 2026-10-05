//! Deterministic presentation fixtures. Prerequisites are assigned by the harness;
//! these snapshots do not claim to replay a complete normal run.
use bevy::prelude::*;
use rogue_commander::game::{
    BondDrawButton, BondId, DraftChoice, DraftState, PlayerBuild, available_bond_cards,
};

#[derive(Resource)]
struct BondChainCapture {
    frame: u32,
    target: Option<BondId>,
}

pub fn configure(world: &mut World, scenario: &str) {
    let target = match scenario {
        "build-chain-journey" => Some(BondId::Journey),
        "build-chain-agility" => Some(BondId::Agility),
        "build-chain-ranger" => Some(BondId::Ranger),
        "build-chain-strength" => Some(BondId::Strength),
        "build-chain-intelligence" => Some(BondId::Intelligence),
        "build-chain-archery" => Some(BondId::Archery),
        "build-chain-warpath" => Some(BondId::Warpath),
        "build-chain-arcanum" => Some(BondId::Arcanum),
        "build-chain-complete" => None,
        _ => panic!("unknown bond chain fixture: {scenario}"),
    };
    let mut build = world.resource_mut::<PlayerBuild>();
    build.owned_bond_cards.fill(false);
    // Complete all unrelated branches and prerequisites. Keep the target and its
    // descendants unowned so descendants remain locked behind the target.
    for family in BondId::ALL {
        let mut ancestor = Some(family);
        let mut depends_on_target = false;
        while let Some(id) = ancestor {
            if Some(id) == target {
                depends_on_target = true;
                break;
            }
            ancestor = id.definition().prerequisite;
        }
        let complete = !depends_on_target;
        if complete {
            for card in family.definition().members {
                build.owned_bond_cards[card.index()] = true;
            }
        }
    }
    assert_eq!(
        build.active_bond_count(),
        0,
        "absorbed families release slots"
    );
    let pool = available_bond_cards(&build);
    if let Some(family) = target {
        assert_eq!(pool.len(), family.definition().members.len());
        assert!(pool.iter().all(|card| card.definition().family == family));
    } else {
        assert!(pool.is_empty(), "fully owned catalog is exhausted");
        assert!(build.owned_bond_cards.iter().all(|owned| *owned));
        assert!(
            BondId::ALL
                .iter()
                .all(|family| build.bond_complete(*family))
        );
        assert!(build.bond_complete(BondId::Ranger));
        assert!(
            BondId::Ranger
                .definition()
                .bonuses
                .iter()
                .all(|bonus| bonus.required <= build.bond(BondId::Ranger))
        );
    }
    println!(
        "[capture] bond-chain fixture {scenario}: explicit harness-owned prerequisites; active=0 pool={pool:?}"
    );
    world.insert_resource(BondChainCapture { frame: 0, target });
}

pub fn install(app: &mut bevy::app::SubApp) {
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
}

fn exercise(
    mut state: ResMut<BondChainCapture>,
    mut buttons: Query<&mut Interaction, With<BondDrawButton>>,
    draft: Res<DraftState>,
    build: Res<PlayerBuild>,
) {
    state.frame += 1;
    if state.frame == 5 && state.target.is_some() {
        for mut interaction in &mut buttons {
            *interaction = Interaction::Pressed;
        }
    }
    if state.frame == 50 {
        if let Some(target) = state.target {
            assert!(draft.root.is_some());
            assert!(!draft.choices.is_empty());
            let mut seen = Vec::new();
            for choice in &draft.choices {
                let DraftChoice::Bond(card) = *choice else {
                    panic!("expected independent card");
                };
                assert_eq!(
                    card.definition().family,
                    target,
                    "actual offers come from unlocked family"
                );
                assert!(!build.owned_bond_cards[card.index()]);
                assert!(!seen.contains(&card), "no duplicate offer");
                seen.push(card);
            }
            println!(
                "[capture] bond-chain actual offers verified: {seen:?}; unique, unowned, prerequisite unlocked"
            );
        } else {
            assert!(draft.root.is_none());
            println!(
                "[capture] bond-chain complete: retained {} unique cards, 0 active family slots, exhausted pool",
                build
                    .owned_bond_cards
                    .iter()
                    .filter(|owned| **owned)
                    .count()
            );
        }
    }
}
