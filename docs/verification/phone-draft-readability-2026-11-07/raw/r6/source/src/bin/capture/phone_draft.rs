//! Controlled real catalog layout plus actual touch input, not earned draw probability.
use bevy::{
    camera::RenderTarget,
    input::touch::{TouchInput, TouchPhase},
    prelude::*,
    text::TextLayoutInfo,
    window::WindowRef,
};
use rogue_commander::game::{
    ActiveSkill, AttackClock, Autoplay, BondCardId, BondId, CampaignMenu, DraftChoice, DraftState,
    GameSession, PlayerBuild, SelectedHero, SkillId, SpawnDirector, UrEvolutionPath,
    capture_open_draft,
};
#[derive(Resource)]
struct Probe {
    frame: u32,
    count: usize,
    saved: Vec<(Entity, RenderTarget)>,
    start: Vec2,
}
fn choices() -> Vec<DraftChoice> {
    vec![
        DraftChoice::Bond(BondCardId::UnstableCatalyst),
        DraftChoice::ReplaceBond(BondId::Sharpshooter),
        DraftChoice::EvolveUr {
            skill: SkillId(27),
            path: UrEvolutionPath::Surge,
        },
        DraftChoice::Evolution {
            skill: SkillId(6),
            tier: 3,
        },
        DraftChoice::ReplaceSkill {
            old: SkillId(6),
            new: SkillId(26),
        },
        DraftChoice::Skill(SkillId(6)),
    ]
}
pub fn configure(world: &mut World, scenario: &str) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.resource_mut::<GameSession>().auto_bond_draws = false;
    world
        .resource_mut::<super::CaptureControls>()
        .suppress_drafts = false;
    world.insert_resource(Probe {
        frame: 0,
        count: scenario.rsplit('-').next().unwrap().parse().unwrap(),
        saved: Vec::new(),
        start: Vec2::ZERO,
    });
}
pub fn install(app: &mut bevy::app::SubApp) {
    app.world_mut().resource_mut::<AttackClock>().0.pause();
    app.world_mut().resource_mut::<ActiveSkill>().remaining = 10000.;
    app.add_systems(First, input)
        .add_systems(PreUpdate, restore.after(bevy::ui::UiSystems::Focus))
        .add_systems(Last, audit);
}
fn named(world: &mut World, name: &str) -> (Vec2, Vec2) {
    world
        .query::<(&Name, &ComputedNode, &UiGlobalTransform)>()
        .iter(world)
        .find(|(n, _, _)| n.as_str() == name)
        .map(|(_, n, t)| (t.translation, n.size()))
        .unwrap_or_else(|| panic!("missing {name}"))
}
fn input(world: &mut World) {
    world.resource_mut::<Probe>().frame += 1;
    let frame = world.resource::<Probe>().frame;
    let count = world.resource::<Probe>().count;
    if frame == 10 {
        capture_open_draft(
            world,
            if count == 3 {
                vec![
                    DraftChoice::Bond(BondCardId::UnstableCatalyst),
                    DraftChoice::EvolveUr {
                        skill: SkillId(27),
                        path: UrEvolutionPath::Surge,
                    },
                    DraftChoice::Skill(SkillId(6)),
                ]
            } else {
                choices()
            },
        );
    }
    // The first gesture starts on a real card. Moving must scroll, never settle.
    let phase = match frame {
        120 | 140 => Some(TouchPhase::Started),
        122 | 124 | 126 | 128 | 142 => Some(TouchPhase::Moved),
        130 | 144 => Some(TouchPhase::Ended),
        200 => Some(TouchPhase::Started),
        204 => Some(TouchPhase::Ended),
        _ => None,
    };
    let Some(phase) = phase else {
        return;
    };
    let window = world
        .query_filtered::<Entity, With<Window>>()
        .single(world)
        .unwrap();
    let position = if frame == 120 || frame == 140 {
        let (center, size) = named(world, "Draft gallery");
        let start = Vec2::new(
            named(world, "Build card 0").0.x,
            center.y + size.y * 0.5 - 16.,
        );
        world.resource_mut::<Probe>().start = start;
        start
    } else if frame == 142 || frame == 144 {
        world.resource::<Probe>().start - Vec2::Y * 100.0
    } else if frame <= 130 {
        world.resource::<Probe>().start - Vec2::Y * (f32::from((frame.min(128) - 120) as u16) * 22.)
    } else {
        named(world, &format!("Build card {}", count - 1)).0
    };
    let scale = world.get::<Window>(window).unwrap().scale_factor();
    world
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(None);
    world.write_message(TouchInput {
        phase,
        position: position / scale,
        window,
        force: None,
        id: 77,
    });
    let targets: Vec<_> = world
        .query_filtered::<(Entity, &RenderTarget), With<Camera>>()
        .iter(world)
        .map(|(e, t)| (e, t.clone()))
        .collect();
    for (e, _) in &targets {
        *world.get_mut::<RenderTarget>(*e).unwrap() =
            RenderTarget::Window(WindowRef::Entity(window));
    }
    world.resource_mut::<Probe>().saved = targets;
    println!("PHONE_TOUCH frame={frame} phase={phase:?} position={position:?} card_count={count}");
}
fn restore(world: &mut World) {
    for (e, target) in std::mem::take(&mut world.resource_mut::<Probe>().saved) {
        *world.get_mut::<RenderTarget>(e).unwrap() = target;
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<Probe>().frame;
    if matches!(frame, 100 | 180) {
        let count = world.resource::<Probe>().count;
        for index in 0..count {
            let (card, card_size) = named(world, &format!("Build card {index}"));
            for part in ["title", "art", "pips"] {
                let (child, child_size) = named(world, &format!("Build {part} {index}"));
                assert!(
                    (child - child_size * 0.5)
                        .cmpge(card - card_size * 0.5 - Vec2::ONE)
                        .all()
                        && (child + child_size * 0.5)
                            .cmple(card + card_size * 0.5 + Vec2::ONE)
                            .all(),
                    "card{index} clips {part}: {child:?}/{child_size:?} card{card:?}/{card_size:?}"
                );
                if frame == 180 && index == count - 1 {
                    let (viewport, viewport_size) = named(world, "Draft gallery");
                    assert!(
                        (child - child_size * 0.5)
                            .cmpge(viewport - viewport_size * 0.5 - Vec2::ONE)
                            .all()
                            && (child + child_size * 0.5)
                                .cmple(viewport + viewport_size * 0.5 + Vec2::ONE)
                                .all(),
                        "last card inner {part} not fully visible after scroll"
                    );
                    println!(
                        "PHONE_INNER_VISIBLE frame={frame} part={part} center={child:?} size={child_size:?}"
                    );
                }
            }
        }
        let (header, header_size) = named(world, "Draft header");
        let (gallery, gallery_size) = named(world, "Draft gallery");
        assert!(header.y + header_size.y * 0.5 <= gallery.y - gallery_size.y * 0.5 + 1.0);
        let mut titles = 0;
        for (name, node, transform, text, layout, font) in world
            .query::<(
                &Name,
                &ComputedNode,
                &UiGlobalTransform,
                Option<&Text>,
                Option<&TextLayoutInfo>,
                Option<&TextFont>,
            )>()
            .iter(world)
        {
            if name.as_str().starts_with("Build title ") {
                let layout = layout.unwrap();
                let text = text.unwrap();
                assert!(!layout.glyphs.is_empty());
                assert!(
                    layout.size.cmple(node.size() + Vec2::ONE).all(),
                    "{name} text overflow {:?}>{:?}",
                    layout.size,
                    node.size()
                );
                println!(
                    "PHONE_TITLE frame={frame} name={name:?} text={:?} position={:?} size={:?} glyph_size={:?} font={:?}",
                    text.0,
                    transform.translation,
                    node.size(),
                    layout.size,
                    font.unwrap().font_size
                );
                titles += 1;
            }
        }
        assert_eq!(titles, count);
        let (center, size) = named(world, &format!("Build card {}", count - 1));
        if frame == 180 {
            let (gallery, area) = named(world, "Draft gallery");
            assert!(
                center.y >= gallery.y - area.y * 0.5 && center.y <= gallery.y + area.y * 0.5,
                "last card center must be reachable after touchscroll: {center:?}/{gallery:?}/{area:?}"
            );
            assert!(
                world.resource::<DraftState>().root.is_some(),
                "drag must not select card"
            );
        }
        assert!(size.min_element() >= 44.);
        println!(
            "PHONE_LAYOUT frame={frame} count={count} last_card_center={center:?} last_card_size={size:?}"
        );
    }
    if frame == 220 {
        assert!(
            world.resource::<DraftState>().root.is_none(),
            "last card actual tap must settle"
        );
        assert_eq!(
            world.resource::<PlayerBuild>().skills[6],
            1,
            "one tap grants exactly one skill rank"
        );
        println!(
            "[capture] PASS phone-draft: full catalog names shaped, drag did not choose, last card tap settled exactly once"
        );
    }
}
