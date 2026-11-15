use bevy::{app::SubApps, prelude::*};
use rogue_commander::game::{
    BondCardId, BondDrawButton, DraftState, GameLanguage, GameSession, PlayerBuild, RunSeed,
    SettingsInputSet,
};

#[derive(Resource)]
struct SettingsCapture {
    scenario: String,
    frame: u32,
    previous_pause: bool,
    cards: Vec<Entity>,
    choices: Vec<String>,
    owned_bond_cards: [bool; BondCardId::COUNT],
    skills: [u8; rogue_commander::game::SkillId::COUNT],
    seed: u64,
}

pub fn install(apps: &mut SubApps, scenario: &str) {
    apps.main
        .world_mut()
        .remove_resource::<rogue_commander::game::Autoplay>();
    apps.main.world_mut().insert_resource(SettingsCapture {
        scenario: scenario.into(),
        frame: 0,
        previous_pause: false,
        cards: Vec::new(),
        choices: Vec::new(),
        owned_bond_cards: [false; BondCardId::COUNT],
        skills: [0; rogue_commander::game::SkillId::COUNT],
        seed: 0,
    });
    apps.main.add_systems(
        PreUpdate,
        exercise
            .after(bevy::ui::UiSystems::Focus)
            .before(SettingsInputSet),
    );
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn exercise(
    mut capture: ResMut<SettingsCapture>,
    mut buttons: Query<(Entity, &Name, &mut Interaction), With<Button>>,
    mut bond_buttons: Query<&mut Interaction, (With<BondDrawButton>, Without<Name>)>,
    mut keyboard: ResMut<ButtonInput<KeyCode>>,
    session: Res<GameSession>,
    draft: Res<DraftState>,
    build: Res<PlayerBuild>,
    seed: Res<RunSeed>,
    language: Res<GameLanguage>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform)>,
    mut scroll: Query<(&Name, &mut ScrollPosition)>,
    windows: Query<&Window>,
    texts: Query<&Text>,
) {
    capture.frame += 1;
    if capture.frame == 5 && capture.scenario == "settings-bond" {
        for mut interaction in &mut bond_buttons {
            *interaction = Interaction::Pressed;
        }
    }
    if capture.scenario == "settings-pause" {
        if capture.frame == 5 {
            keyboard.press(KeyCode::Escape);
        }
        if capture.frame == 6 {
            keyboard.release(KeyCode::Escape);
        }
    }
    if capture.frame == 9 {
        capture.previous_pause = session.paused;
        capture.cards = buttons
            .iter()
            .filter(|(_, name, _)| {
                name.as_str().starts_with("Commander card")
                    || name.as_str().starts_with("Build card")
            })
            .map(|(entity, _, _)| entity)
            .collect();
        capture.choices = draft.choices.iter().map(|choice| choice.title()).collect();
        capture.owned_bond_cards = build.owned_bond_cards;
        capture.skills = build.skills;
        capture.seed = seed.0;
        if capture.scenario == "settings-bond" {
            assert!(draft.root.is_some(), "bond draft opened before settings");
        }
    }
    let close = capture.scenario != "settings-recruit" && capture.scenario != "settings-open";
    let action = match capture.frame {
        10 => Some("Settings open"),
        20 => Some(if capture.scenario == "settings-english" {
            "Settings English"
        } else {
            "Settings Chinese"
        }),
        60 if close => Some("Settings close"),
        _ => None,
    };
    if let Some(target) = action {
        let mut found = false;
        for (_, name, mut interaction) in &mut buttons {
            if name.as_str() == target {
                *interaction = Interaction::Pressed;
                found = true;
            }
        }
        assert!(found, "missing settings action {target}");
        println!("[settings capture] pressed {target}");
    }
    if capture.frame == 40 {
        assert!(session.paused, "settings pauses simulation");
        let window = windows.single().unwrap();
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        let mut count = 0;
        for (name, node, transform) in &layout {
            if !matches!(
                name.as_str(),
                "Settings English"
                    | "Settings Chinese"
                    | "Settings close"
                    | "Settings chapters"
                    | "Settings quit"
                    | "Settings display toggle"
            ) || node.size().min_element() == 0.0
            {
                continue;
            }
            let half = node.size() * 0.5;
            assert!(
                (transform.translation - half)
                    .cmpge(Vec2::splat(-1.0))
                    .all()
                    && (transform.translation + half)
                        .cmple(viewport + Vec2::ONE)
                        .all(),
                "{name} clipped"
            );
            assert!(
                node.size().min_element() >= 44.0,
                "{name} touch target too small"
            );
            if matches!(name.as_str(), "Settings English" | "Settings Chinese") {
                let (_, body, pose) = layout
                    .iter()
                    .find(|(name, _, _)| name.as_str() == "Settings scroll body")
                    .unwrap();
                let body_rect = Rect::from_center_size(pose.translation, body.size());
                let rect = Rect::from_center_size(transform.translation, node.size());
                assert!(
                    rect.min.y >= body_rect.min.y - 1. && rect.max.y <= body_rect.max.y + 1.,
                    "language selectors visible before scrolling"
                );
            }

            count += 1;
        }
        assert!(count >= 3, "language selectors and back button visible");
    }
    // Exercise the real scroll container, rather than accepting clipped controls.
    if capture.frame == 45 {
        for (name, mut position) in &mut scroll {
            if name.as_str() == "Settings scroll body" {
                position.0.y = 10000.;
            }
        }
    }
    if capture.frame == 55 {
        let (_, body, body_transform) = layout
            .iter()
            .find(|(name, _, _)| name.as_str() == "Settings scroll body")
            .expect("settings scroll body");
        let bounds = Rect::from_center_size(body_transform.translation, body.size());
        let max_scroll = (body.content_size().y - body.size().y + body.scrollbar_size.y).max(0.)
            * body.inverse_scale_factor();
        let (_, position) = scroll
            .iter()
            .find(|(name, _)| name.as_str() == "Settings scroll body")
            .unwrap();
        assert!(
            position.0.y <= max_scroll + 0.01 && position.0.y >= 0.,
            "requested scroll must stay inside real content: {} / {max_scroll}",
            position.0.y
        );
        println!(
            "[settings capture] scroll requested={} rendered={} content={:?} viewport={:?} max={max_scroll}",
            position.0.y,
            body.scroll_position.y,
            body.content_size(),
            body.size()
        );
        let mut visible_controls = 0;
        let mut audio_reached = false;
        for (name, node, transform) in &layout {
            if matches!(
                name.as_str(),
                "Settings English"
                    | "Settings Chinese"
                    | "Settings music down"
                    | "Settings music up"
                    | "Settings effects down"
                    | "Settings effects up"
                    | "Settings mute"
            ) && node.size().min_element() > 0.
            {
                let rect = Rect::from_center_size(transform.translation, node.size());
                if rect.intersect(bounds).height() >= 22. {
                    visible_controls += 1;
                }
                if name.as_str() == "Settings mute" {
                    audio_reached =
                        rect.min.y >= bounds.min.y - 1. && rect.max.y <= bounds.max.y + 1.;
                }
            }
            if name.as_str() == "Settings tutorial" && node.size().min_element() > 0. {
                let rect = Rect::from_center_size(transform.translation, node.size());
                assert!(
                    rect.min.y >= bounds.min.y - 1. && rect.max.y <= bounds.max.y + 1.,
                    "tutorial must be reachable after scrolling: {rect:?} / {bounds:?}"
                );
                assert!(node.size().min_element() >= 44., "tutorial touch target");
            }
        }
        assert!(
            visible_controls > 0,
            "scrolling must never produce an empty settings body"
        );
        assert!(
            audio_reached,
            "bottom audio control must be reachable after scrolling"
        );
        let (_, node, transform) = layout
            .iter()
            .find(|(name, _, _)| name.as_str() == "Settings close")
            .expect("fixed back action");
        let viewport = windows.single().unwrap().physical_size().as_vec2();
        let rect = Rect::from_center_size(transform.translation, node.size());
        assert!(
            rect.min.cmpge(Vec2::ZERO).all() && rect.max.cmple(viewport + Vec2::ONE).all(),
            "back remains reachable while scrolling"
        );
        println!(
            "[settings capture] PASS bounded scroll reaches audio and optional tutorial; fixed back remains visible"
        );
    }
    if capture.frame == 90 {
        let expected = if capture.scenario == "settings-english" {
            GameLanguage::English
        } else {
            GameLanguage::SimplifiedChinese
        };
        assert_eq!(*language, expected);
        assert_eq!(
            session.paused,
            if close { capture.previous_pause } else { true }
        );
        assert_eq!(capture.seed, seed.0);
        assert_eq!(capture.owned_bond_cards, build.owned_bond_cards);
        assert_eq!(capture.skills, build.skills);
        assert_eq!(
            capture.choices,
            draft
                .choices
                .iter()
                .map(|choice| choice.title())
                .collect::<Vec<_>>()
        );
        let cards: Vec<_> = buttons
            .iter()
            .filter(|(_, name, _)| {
                name.as_str().starts_with("Commander card")
                    || name.as_str().starts_with("Build card")
            })
            .map(|(entity, _, _)| entity)
            .collect();
        assert_eq!(
            capture.cards, cards,
            "live translation must retain existing card entities"
        );
        if !close && expected == GameLanguage::SimplifiedChinese {
            assert!(
                texts.iter().any(|text| text.0 == "设置"),
                "Chinese settings heading visible"
            );
        }
        if capture.scenario == "settings-recruit-close" {
            assert!(
                !texts
                    .iter()
                    .any(|text| text.0 == "Choose a basic commander")
            );
        }
        println!(
            "[settings capture] language applied; seed/build/offers/cards preserved; pause restored correctly"
        );
    }
}
