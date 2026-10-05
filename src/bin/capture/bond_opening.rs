//! Acquires the opening Journey cards through the actual draw/hover/immediate-click UI.
use bevy::prelude::*;
use rogue_commander::game::{
    Autoplay, BondCardId, BondDrawButton, BondId, DraftChoice, DraftState, GameSession,
    PlayerBuild, SelectedHero, SpawnDirector, available_bond_cards,
};

#[derive(Resource, Default)]
struct OpeningCapture {
    frame: u32,
}

pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    {
        let mut session = world.resource_mut::<GameSession>();
        session.elapsed = 20.0;
        session.wood = 100;
        session.skill_rerolls = 3;
        session.skill_points = 0;
        session.auto_bond_draws = false;
    }
    world
        .resource_mut::<PlayerBuild>()
        .owned_bond_cards
        .fill(false);
    world.init_resource::<OpeningCapture>();
    println!(
        "[capture] bond-opening: fresh collection, actual draw/refresh/later/confirm controls"
    );
}

pub fn install(app: &mut bevy::app::SubApp) {
    app.add_systems(PreUpdate, exercise.after(bevy::ui::UiSystems::Focus));
}

#[allow(clippy::type_complexity)]
fn exercise(
    mut state: ResMut<OpeningCapture>,
    mut buttons: Query<(Option<&Name>, &mut Interaction, Has<BondDrawButton>), With<Button>>,
    draft: Res<DraftState>,
    build: Res<PlayerBuild>,
    session: Res<GameSession>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
    windows: Query<&Window>,
) {
    state.frame += 1;
    let draw = matches!(state.frame, 5 | 55 | 105 | 155 | 250);
    let action = match state.frame {
        20 => Some("Build refresh"),
        40 => Some("Build later"),
        70 | 120 | 170 => Some("Build card 0"),
        _ => None,
    };
    let release = matches!(
        state.frame,
        6 | 21 | 41 | 56 | 71 | 81 | 106 | 121 | 131 | 156 | 171 | 181 | 251
    );
    let mut pressed = false;
    for (name, mut interaction, bond_draw) in &mut buttons {
        if (draw && bond_draw)
            || action.is_some_and(|target| name.is_some_and(|name| name.as_str() == target))
        {
            *interaction = Interaction::Pressed;
            pressed = true;
        } else if release {
            *interaction = Interaction::None;
        }
    }
    if draw || action.is_some() {
        assert!(
            pressed,
            "opening action missing: draw={draw}, action={action:?}"
        );
    }
    if matches!(state.frame, 15 | 30 | 60 | 115 | 165) {
        let acquired = if state.frame == 165 {
            2
        } else if state.frame == 115 {
            1
        } else {
            0
        };
        let expected: Vec<_> = [
            BondCardId::JourneyStrength,
            BondCardId::JourneyAgility,
            BondCardId::JourneyIntelligence,
        ]
        .into_iter()
        .skip(acquired)
        .map(DraftChoice::Bond)
        .collect();
        assert_eq!(
            draft.choices, expected,
            "opening offers remain in reference order"
        );
        assert!(draft.root.is_some() && session.paused);
        assert_eq!(build.bond(BondId::Journey) as usize, acquired);
        let window = windows.single().unwrap();
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        for index in 0..expected.len() {
            let label = format!("Build card {index}");
            let (_, node, transform) = layout
                .iter()
                .find(|(name, _, _)| name.as_str() == label)
                .expect("offered card has a UI node");
            let min = transform.translation - node.size() * 0.5;
            let max = transform.translation + node.size() * 0.5;
            assert!(node.size().min_element() > 0.0);
            assert!(
                min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
                "opening card outside viewport"
            );
        }
        println!(
            "[capture] {} opening offers in fixed order; owned {acquired}/3; cards fit viewport",
            expected.len()
        );
    }
    if state.frame == 45 {
        assert!(draft.root.is_none() && !session.paused);
        assert_eq!(build.bond(BondId::Journey), 0);
        println!("[capture] choose-later grants no card; reopening preserves opening offers");
    }
    if matches!(state.frame, 90 | 140 | 190) {
        let acquired = match state.frame {
            90 => 1,
            140 => 2,
            _ => 3,
        };
        assert_eq!(build.bond(BondId::Journey), acquired);
        assert_eq!(
            build
                .owned_bond_cards
                .iter()
                .filter(|owned| **owned)
                .count(),
            acquired as usize
        );
        assert!(draft.root.is_none() && !session.paused);
        println!(
            "[capture] actual immediate click acquired opening card {acquired}/3 exactly once"
        );
    }
    if state.frame == 260 {
        assert!(build.bond_complete(BondId::Journey));
        let pool = available_bond_cards(&build);
        for family in [BondId::Strength, BondId::Agility, BondId::Intelligence] {
            assert!(pool.iter().any(|card| card.family() == family));
        }
        assert!(!draft.choices.is_empty() && draft.root.is_some());
        assert!(draft.choices.iter().all(|choice| matches!(choice, DraftChoice::Bond(card) if card.family() != BondId::Journey && !build.owns_bond_card(*card))));
        println!(
            "[capture] PASS: opening 3 -> 2 -> 1, refresh/later preserved gate, all three progression pools unlocked after completion"
        );
    }
}
