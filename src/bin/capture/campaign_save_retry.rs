//! A genuine filesystem failure, repaired through the campaign's real retry button.
use bevy::{app::SubApp, prelude::*};
use rogue_commander::game::{
    ArchiveOutcome, Autoplay, CampaignMenu, CampaignProgress, CampaignReward, GameLanguage,
    GameSession, RunReward, SpawnDirector,
};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
#[derive(Resource)]
struct RetryProbe {
    frame: u32,
    path: PathBuf,
    blocked: PathBuf,
    backup: PathBuf,
    original: Vec<u8>,
    currency: u32,
    upgrades: [u8; 6],
    reward: RunReward,
}
pub fn configure(world: &mut World) {
    world.remove_resource::<Autoplay>();
    world.resource_mut::<CampaignMenu>().0 = true;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let base =
        std::env::temp_dir().join(format!("rogue-save-retry-{}-{nonce}", std::process::id()));
    fs::create_dir(&base).unwrap();
    let blocked = base.join("save");
    let backup = base.join("backup");
    let path = blocked.join("progress.save");
    let mut progress = CampaignProgress::load_from_path(&path).unwrap();
    progress.persist().unwrap();
    let original = fs::read(&path).unwrap();
    fs::rename(&blocked, &backup).unwrap();
    fs::write(&blocked, "temporary capture filesystem obstruction").unwrap();
    let mut rewards = CampaignReward::default();
    let reward = rewards
        .settle(&mut progress, 0, true, 200, ArchiveOutcome::TimedOut)
        .unwrap();
    assert!(progress.purchase(0));
    assert!(progress.persist().is_err());
    assert!(progress.can_retry_save());
    let probe = RetryProbe {
        frame: 0,
        path,
        blocked,
        backup,
        original,
        currency: progress.currency,
        upgrades: progress.upgrades,
        reward,
    };
    {
        let mut session = world.resource_mut::<GameSession>();
        session.finished = true;
        session.victory = true;
        session.total_kills = 200;
        session.paused = true;
    }
    world.insert_resource(progress);
    world.insert_resource(rewards);
    world.insert_resource(probe);
}
pub fn install(app: &mut SubApp) {
    app.world_mut().remove_resource::<Autoplay>();
    app.add_systems(PreUpdate, input.after(bevy::ui::UiSystems::Focus));
    app.add_systems(Last, audit);
}
fn input(world: &mut World) {
    world.resource_mut::<SpawnDirector>().enabled = false;
    let frame = {
        let mut p = world.resource_mut::<RetryProbe>();
        p.frame += 1;
        p.frame
    };
    let pressed: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in pressed {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if frame == 30 {
        let p = world.resource::<RetryProbe>();
        fs::remove_file(&p.blocked).unwrap();
        fs::rename(&p.backup, &p.blocked).unwrap();
    }
    if frame == 40 {
        let button = world
            .query::<(Entity, &Name, &Interaction)>()
            .iter(world)
            .find_map(|(e, n, _)| (n.as_str() == "Campaign retry save").then_some(e))
            .expect("actual retry-save button");
        *world.get_mut::<Interaction>(button).unwrap() = Interaction::Pressed;
        println!("CAMPAIGN_SAVE_RETRY_REAL_CLICK");
    }
}
fn audit(world: &mut World) {
    let frame = world.resource::<RetryProbe>().frame;
    if ![20, 50, 80].contains(&frame) {
        return;
    }
    let retry = world
        .query::<&Name>()
        .iter(world)
        .any(|n| n.as_str() == "Campaign retry save");
    let chinese = world.resource::<GameLanguage>().is_chinese();
    let label = if chinese {
        "重试保存"
    } else {
        "Retry save"
    };
    let warning = if chinese {
        "保存失败"
    } else {
        "Saving failed"
    };
    let text: Vec<String> = world
        .query::<&Text>()
        .iter(world)
        .map(|t| t.0.clone())
        .collect();
    let p = world.resource::<RetryProbe>();
    let progress = world.resource::<CampaignProgress>();
    assert_eq!(progress.currency, p.currency);
    assert_eq!(progress.upgrades, p.upgrades);
    assert_eq!(world.resource::<CampaignReward>().last, Some(p.reward));
    assert!(world.resource::<GameSession>().paused);
    if frame == 20 {
        assert!(progress.can_retry_save() && retry);
        assert!(
            text.iter().any(|t| t == label),
            "retry label must match current locale"
        );
        assert!(
            text.iter().any(|t| t.contains(warning)),
            "real localized failure notice"
        );
        assert_eq!(
            fs::read(p.backup.join("progress.save")).unwrap(),
            p.original
        );
        println!(
            "CAMPAIGN_SAVE_RETRY_FAILURE_VISIBLE locale={} currency={}",
            if chinese { "zh" } else { "en" },
            p.currency
        );
    } else {
        assert!(progress.last_error.is_none() && !progress.can_retry_save() && !retry);
        assert!(
            !text.iter().any(|t| t.contains(warning)),
            "failure warning clears after actual successful retry"
        );
        let loaded = CampaignProgress::load_from_path(&p.path).unwrap();
        assert_eq!(loaded.currency, p.currency);
        assert_eq!(loaded.upgrades, p.upgrades);
        assert_eq!(loaded.cleared, progress.cleared);
        assert_eq!(loaded.unlocked, progress.unlocked);
        println!(
            "CAMPAIGN_SAVE_RETRY_PERSISTED frame={frame} currency={} upgrades={:?}",
            p.currency, p.upgrades
        );
        if frame == 80 {
            println!(
                "CAMPAIGN_SAVE_RETRY_PASS no duplicate rewards; isolated path={}",
                p.path.display()
            );
        }
    }
}
