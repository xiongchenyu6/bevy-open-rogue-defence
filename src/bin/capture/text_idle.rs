//! Measure production UI change ticks and shaping work while gameplay is paused.
use bevy::{app::SubApp, prelude::*, text::TextLayoutInfo};
use rogue_commander::game::{
    AudioPreferences, Autoplay, CampaignMenu, GameLanguage, GameSession, SelectedHero,
    SettingsDepartureInputSet, SpawnDirector,
};
#[derive(Resource, Default)]
struct IdleProbe {
    frame: u32,
    counts: [[u64; 3]; 3],
    music_label: Option<Entity>,
}
pub fn install(app: &mut SubApp) {
    let world = app.world_mut();
    world.remove_resource::<Autoplay>();
    world.resource_mut::<SelectedHero>().locked = true;
    world.resource_mut::<CampaignMenu>().0 = false;
    world.resource_mut::<GameSession>().paused = true;
    world.resource_mut::<SpawnDirector>().enabled = false;
    world.insert_resource(GameLanguage::English);
    {
        let mut audio = world.resource_mut::<AudioPreferences>();
        audio.music = 40;
        audio.effects = 70;
    }
    world.init_resource::<IdleProbe>();
    app.add_systems(
        PreUpdate,
        input
            .after(bevy::ui::UiSystems::Focus)
            .before(SettingsDepartureInputSet),
    );
    app.add_systems(Last, audit);
}
fn input(world: &mut World) {
    let frame = {
        let mut c = world.resource_mut::<IdleProbe>();
        c.frame += 1;
        c.frame
    };
    let pressed: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in pressed {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if frame == 5 {
        world.insert_resource(GameLanguage::SimplifiedChinese);
    }
    if frame == 95 {
        world.insert_resource(GameLanguage::English);
    }
    let name = match frame {
        100 => "Settings open",
        110 => "Settings Chinese",
        210 => "Settings music up",
        220 => "Settings English",
        _ => return,
    };
    let e = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == name).then_some(e))
        .unwrap_or_else(|| panic!("missing actual UI input {name}"));
    *world.get_mut::<Interaction>(e).unwrap() = Interaction::Pressed;
    println!("TEXT_IDLE_ACTION {frame} {name}");
}
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn audit(
    mut probe: ResMut<IdleProbe>,
    session: Res<GameSession>,
    language: Res<GameLanguage>,
    audio: Res<AudioPreferences>,
    texts: Query<(Entity, &Text, Option<&Name>), Changed<Text>>,
    fonts: Query<Entity, Changed<TextFont>>,
    layouts: Query<Entity, Changed<TextLayoutInfo>>,
    all_texts: Query<(Entity, &Text)>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
) {
    let frame = probe.frame;
    assert!(
        session.paused,
        "the full text-idle experiment must remain paused"
    );
    let phase = match frame {
        30..=90 => Some(0),
        140..=200 => Some(1),
        250..=310 => Some(2),
        _ => None,
    };
    if let Some(phase) = phase {
        let counts = [
            texts.iter().count() as u64,
            fonts.iter().count() as u64,
            layouts.iter().count() as u64,
        ];
        for (total, count) in probe.counts[phase].iter_mut().zip(counts) {
            *total += count;
        }
        if [30, 31, 140, 141, 250, 251].contains(&frame) {
            println!(
                "TEXT_IDLE_SAMPLE frame={frame} phase={phase} text={} font={} layout={}",
                counts[0], counts[1], counts[2]
            );
            for (e, text, name) in texts.iter().take(20) {
                let mut label = name.map(|n| n.as_str());
                let mut ancestor = parents.get(e).ok().map(ChildOf::parent);
                while label.is_none() {
                    let Some(parent) = ancestor else {
                        break;
                    };
                    label = names.get(parent).ok().map(|n| n.as_str());
                    ancestor = parents.get(parent).ok().map(ChildOf::parent);
                }
                println!("TEXT_IDLE_CHANGED {e:?} name={label:?} text={:?}", text.0);
            }
        }
    }
    if frame == 90 {
        assert_eq!(*language, GameLanguage::SimplifiedChinese);
    }
    if frame == 140 {
        assert_eq!(*language, GameLanguage::SimplifiedChinese);
        assert!(all_texts.iter().any(|(_, t)| t.0 == "设置"));
    }
    if frame == 200 {
        probe.music_label = all_texts
            .iter()
            .find_map(|(e, t)| (t.0 == "40%").then_some(e));
        assert!(
            probe.music_label.is_some(),
            "actual music value label is present"
        );
    }
    if frame == 210 {
        assert_eq!(audio.music, 50);
        let entity = probe.music_label.unwrap();
        assert_eq!(all_texts.get(entity).unwrap().1.0, "50%");
        assert!(
            texts.get(entity).is_ok(),
            "real volume input marks its displayed value changed"
        );
        println!("TEXT_IDLE_VOLUME_CHANGED");
    }
    if frame == 220 {
        assert_eq!(*language, GameLanguage::English);
        assert!(texts.iter().any(|(_, t, _)| t.0 == "Settings"));
        println!("TEXT_IDLE_LANGUAGE_CHANGED");
    }
    if frame == 310 {
        assert!(all_texts.iter().any(|(_, t)| t.0 == "Settings"));
        for (phase, counts) in probe.counts.iter().enumerate() {
            println!(
                "TEXT_IDLE_STEADY phase={phase} frames=61 changed_text={} changed_font={} changed_layout={}",
                counts[0], counts[1], counts[2]
            );
        }
        assert_eq!(
            probe.counts, [[0; 3]; 3],
            "paused steady UI must not rewrite text, font or layouts"
        );
        println!(
            "TEXT_IDLE_PASS steady_text_font_layout=0 volume_changes=true language_changes=true"
        );
    }
}
