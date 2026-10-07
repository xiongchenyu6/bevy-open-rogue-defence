//! Persistent campaign rewards and bounded permanent upgrades, separate from run state.
use super::{ArchiveOutcome, ArchiveState, GameSession, HeroVitals, PlayerBuild, SelectedHero};
use bevy::prelude::*;

pub const MAX_UPGRADE_LEVEL: u8 = 10;
pub const UPGRADE_PER_LEVEL: [f32; 6] = [2., 2., 2., 30., 3., 4.];
const CHAPTERS: usize = 6;

#[derive(Clone, Debug, Default)]
enum Storage {
    #[default]
    Memory,
    #[cfg(not(target_arch = "wasm32"))]
    File(std::path::PathBuf),
    #[cfg(target_arch = "wasm32")]
    Browser,
}
#[derive(Resource, Clone, Debug)]
pub struct CampaignProgress {
    /// Number of available chapters, indexed from zero by the campaign runner.
    pub unlocked: u8,
    pub currency: u32,
    pub upgrades: [u8; 6],
    pub cleared: [bool; CHAPTERS],
    pub last_error: Option<String>,
    storage: Storage,
}
impl Default for CampaignProgress {
    fn default() -> Self {
        Self {
            unlocked: 1,
            currency: 0,
            upgrades: [0; 6],
            cleared: [false; CHAPTERS],
            last_error: None,
            storage: Storage::Memory,
        }
    }
}
impl CampaignProgress {
    pub fn can_play(&self, chapter: usize) -> bool {
        chapter < usize::from(self.unlocked) && chapter < CHAPTERS
    }
    pub fn upgrade_cost(&self, index: usize) -> Option<u32> {
        let level = u32::from(*self.upgrades.get(index)?);
        (level < u32::from(MAX_UPGRADE_LEVEL)).then_some(15 + 10 * level + 5 * level * level)
    }
    pub fn purchase(&mut self, index: usize) -> bool {
        let Some(cost) = self.upgrade_cost(index) else {
            return false;
        };
        if self.currency < cost {
            return false;
        }
        self.currency -= cost;
        self.upgrades[index] += 1;
        true
    }
    pub fn save(&self) -> Result<(), String> {
        match &self.storage {
            Storage::Memory => self.last_error.clone().map_or(Ok(()), Err),
            #[cfg(not(target_arch = "wasm32"))]
            Storage::File(path) => save_file(path, &self.encode()),
            #[cfg(target_arch = "wasm32")]
            Storage::Browser => browser_storage()?
                .set_item("rogue_commander_campaign_v1", &self.encode())
                .map_err(|_| "Could not save campaign progress in browser storage".into()),
        }
    }
    /// Loading errors keep storage detached, so retry cannot overwrite an unread save.
    pub fn can_retry_save(&self) -> bool {
        self.last_error.is_some() && !matches!(self.storage, Storage::Memory)
    }
    pub fn persist(&mut self) -> Result<(), String> {
        let result = self.save();
        self.last_error = result.as_ref().err().cloned();
        result
    }
    pub fn retry_save(&mut self) -> Result<(), String> {
        if !self.can_retry_save() {
            return Err("Campaign progress is temporary for this session".into());
        }
        self.persist()
    }
    fn encode(&self) -> String {
        let body = format!(
            "RCAMPAIGN1\n{}\n{}\n{}\n{}\n",
            self.unlocked,
            self.currency,
            self.cleared
                .iter()
                .map(|clear| if *clear { '1' } else { '0' })
                .collect::<String>(),
            self.upgrades
                .iter()
                .map(u8::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
        format!("{body}{:016x}\n", checksum(body.as_bytes()))
    }
    fn decode(text: &str) -> Result<Self, String> {
        let lines = text.lines().collect::<Vec<_>>();
        let invalid = || "Campaign save is invalid; existing file was preserved".to_owned();
        if lines.len() != 6 || lines[0] != "RCAMPAIGN1" {
            return Err(invalid());
        }
        let body = format!("{}\n", lines[..5].join("\n"));
        if u64::from_str_radix(lines[5], 16).ok() != Some(checksum(body.as_bytes())) {
            return Err(invalid());
        }
        let unlocked = lines[1].parse::<u8>().map_err(|_| invalid())?;
        let currency = lines[2].parse::<u32>().map_err(|_| invalid())?;
        let digits = lines[3].as_bytes();
        if !(1..=6).contains(&unlocked)
            || digits.len() != CHAPTERS
            || !digits.iter().all(|b| matches!(b, b'0' | b'1'))
        {
            return Err(invalid());
        }
        let cleared = std::array::from_fn(|i| digits[i] == b'1');
        let clear_count = cleared.iter().take_while(|clear| **clear).count();
        if cleared[clear_count..].iter().any(|clear| *clear)
            || usize::from(unlocked) != (clear_count + 1).min(CHAPTERS)
        {
            return Err(invalid());
        }
        let upgrades = lines[4]
            .split(',')
            .map(str::parse::<u8>)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid())?;
        if upgrades.len() != 6 || upgrades.iter().any(|level| *level > MAX_UPGRADE_LEVEL) {
            return Err(invalid());
        }
        Ok(Self {
            unlocked,
            currency,
            cleared,
            upgrades: upgrades.try_into().map_err(|_| invalid())?,
            ..default()
        })
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_path(path: impl Into<std::path::PathBuf>) -> Result<Self, String> {
        let path = path.into();
        let mut progress = match std::fs::read_to_string(&path) {
            Ok(text) => Self::decode(&text)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(format!("Could not read campaign save: {error}")),
        };
        progress.storage = Storage::File(path);
        Ok(progress)
    }
}
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunReward {
    pub chapter: usize,
    pub victory: bool,
    pub kills: u32,
    /// Total shards, including the first-clear and imprint bonuses.
    pub reward: u32,
    pub first_clear_bonus: u32,
    /// Extra shards for shattering the archive imprint before its timer ran out.
    pub imprint_bonus: u32,
}
/// Shards for shattering the archive imprint; `chapter` is zero-based.
const fn imprint_shards(chapter: usize) -> u32 {
    15 + 5 * chapter as u32
}
#[derive(Resource, Default)]
pub struct CampaignReward {
    pub last: Option<RunReward>,
    settled: bool,
}
impl CampaignReward {
    pub fn settle(
        &mut self,
        progress: &mut CampaignProgress,
        chapter: usize,
        victory: bool,
        kills: u32,
        archive: ArchiveOutcome,
    ) -> Option<RunReward> {
        if self.settled || !progress.can_play(chapter) {
            return None;
        }
        self.settled = true;
        let first_clear_bonus = if victory && !progress.cleared[chapter] {
            40 + chapter as u32 * 10
        } else {
            0
        };
        let imprint_bonus = if archive == ArchiveOutcome::Captured {
            imprint_shards(chapter)
        } else {
            0
        };
        let reward = 5
            + (kills / 20).min(100)
            + if victory { 20 + chapter as u32 * 5 } else { 0 }
            + first_clear_bonus
            + imprint_bonus;
        progress.currency = progress.currency.saturating_add(reward);
        if victory {
            progress.cleared[chapter] = true;
            progress.unlocked = progress.unlocked.max(((chapter + 2).min(CHAPTERS)) as u8);
        }
        let result = RunReward {
            chapter,
            victory,
            kills,
            reward,
            first_clear_bonus,
            imprint_bonus,
        };
        self.last = Some(result);
        Some(result)
    }
}
#[derive(Resource, Default)]
struct AppliedPermanentBonus(bool);
pub(crate) fn reset_run_progress(world: &mut World) {
    world.insert_resource(CampaignReward::default());
    world.insert_resource(AppliedPermanentBonus::default());
}
pub(crate) struct CampaignProgressPlugin;
impl Plugin for CampaignProgressPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CampaignProgress>()
            .init_resource::<CampaignReward>()
            .init_resource::<AppliedPermanentBonus>()
            .add_systems(Startup, load)
            .add_systems(
                Update,
                apply_new_run
                    .after(super::hero::synchronize_hero_growth)
                    .before(super::combat::CombatSet),
            )
            .add_systems(Last, settle_finished_run);
    }
}
fn apply_new_run(
    progress: Res<CampaignProgress>,
    selected: Res<SelectedHero>,
    mut applied: ResMut<AppliedPermanentBonus>,
    mut build: ResMut<PlayerBuild>,
    mut vitals: ResMut<HeroVitals>,
) {
    if !selected.locked || applied.0 {
        return;
    }
    applied.0 = true;
    apply_permanent(&progress, &mut build, &mut vitals);
}
fn apply_permanent(progress: &CampaignProgress, build: &mut PlayerBuild, vitals: &mut HeroVitals) {
    let gains: [f32; 6] =
        std::array::from_fn(|i| f32::from(progress.upgrades[i]) * UPGRADE_PER_LEVEL[i]);
    let primary = vitals.primary_attribute();
    vitals.strength += gains[0];
    vitals.agility += gains[1];
    vitals.intelligence += gains[2];
    let health = gains[0] * 14. * vitals.command_health_multiplier + gains[3];
    vitals.max_health += health;
    vitals.health += health;
    vitals.max_energy += gains[2] * 8.;
    vitals.energy += gains[2] * 8.;
    vitals.permanent_spell_power += gains[5];
    build.attack_damage += vitals.primary_attribute() - primary + gains[4];
    build.attacks_per_second += gains[1] * 0.028;
    build.move_speed += gains[1] * 0.035;
    build.projectile_speed += gains[2] * 0.08;
    build.attack_range += gains[2] * 0.025;
}
fn settle_finished_run(
    session: Res<GameSession>,
    archive: Res<ArchiveState>,
    run: Res<super::campaign::CampaignRun>,
    mut progress: ResMut<CampaignProgress>,
    mut reward: ResMut<CampaignReward>,
) {
    if !session.finished || reward.settled {
        return;
    }
    if reward
        .settle(
            &mut progress,
            run.stage,
            session.victory,
            session.total_kills,
            archive.outcome,
        )
        .is_some()
        && let Err(error) = progress.persist()
    {
        warn!("{error}");
        progress.last_error = Some(error);
    }
}
fn load(mut progress: ResMut<CampaignProgress>, capture: Option<Res<super::CaptureTarget>>) {
    #[cfg(not(target_arch = "wasm32"))]
    let loaded = {
        let override_path = std::env::var_os("ROGUE_CAMPAIGN_SAVE").map(std::path::PathBuf::from);
        if capture.is_some() && override_path.is_none() {
            return;
        }
        let path = override_path.or_else(default_path);
        path.ok_or_else(|| "No writable configuration directory for campaign progress".to_owned())
            .and_then(CampaignProgress::load_from_path)
    };
    #[cfg(target_arch = "wasm32")]
    let loaded = {
        if capture.is_some() {
            return;
        }
        browser_storage()
            .and_then(|storage| {
                storage
                    .get_item("rogue_commander_campaign_v1")
                    .map_err(|_| "Could not read campaign progress".into())
            })
            .and_then(|text| {
                text.map_or_else(
                    || Ok(CampaignProgress::default()),
                    |text| CampaignProgress::decode(&text),
                )
            })
            .map(|mut progress| {
                progress.storage = Storage::Browser;
                progress
            })
    };
    match loaded {
        Ok(loaded) => *progress = loaded,
        Err(error) => {
            warn!("{error}");
            progress.last_error = Some(error);
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn default_path() -> Option<std::path::PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(std::path::PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .map(|root| root.join("rogue-commander").join("campaign-v1.save"))
}
#[cfg(not(target_arch = "wasm32"))]
fn save_file(path: &std::path::Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    static SERIAL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(std::path::Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create campaign directory: {error}"))?;
    let temp = parent.join(format!(
        ".campaign-{}-{}.tmp",
        std::process::id(),
        SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(|error| format!("Could not save campaign progress: {error}"))
}
#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("Browser window unavailable")?
        .local_storage()
        .map_err(|_| "Browser storage unavailable")?
        .ok_or_else(|| "Browser storage disabled".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn victory_unlocks_next_chapter_and_settles_only_once() {
        let mut progress = CampaignProgress::default();
        let mut result = CampaignReward::default();
        let reward = result
            .settle(&mut progress, 0, true, 200, ArchiveOutcome::TimedOut)
            .unwrap();
        assert_eq!(reward.reward, 75);
        assert_eq!(reward.first_clear_bonus, 40);
        assert_eq!(progress.unlocked, 2);
        assert!(progress.cleared[0]);
        assert!(
            result
                .settle(&mut progress, 0, true, 200, ArchiveOutcome::TimedOut)
                .is_none()
        );
        assert_eq!(progress.currency, 75);
        let again = CampaignReward::default()
            .settle(&mut progress, 0, true, 200, ArchiveOutcome::TimedOut)
            .unwrap();
        assert_eq!(again.first_clear_bonus, 0);
        assert_eq!(again.reward, 35);
    }
    #[test]
    fn shattering_the_imprint_adds_chapter_scaled_shards_to_the_victory_reward() {
        for chapter in 0..CHAPTERS {
            let mut progress = CampaignProgress {
                unlocked: 6,
                cleared: [true; CHAPTERS],
                ..default()
            };
            let reward = CampaignReward::default()
                .settle(&mut progress, chapter, true, 200, ArchiveOutcome::Captured)
                .unwrap();
            let bonus = 15 + 5 * chapter as u32;
            assert_eq!(reward.imprint_bonus, bonus);
            assert_eq!(reward.reward, 35 + 5 * chapter as u32 + bonus);
            assert_eq!(progress.currency, reward.reward);
        }
    }
    #[test]
    fn letting_the_imprint_timer_expire_grants_no_imprint_shards() {
        let mut progress = CampaignProgress {
            unlocked: 6,
            cleared: [true; CHAPTERS],
            ..default()
        };
        let reward = CampaignReward::default()
            .settle(&mut progress, 5, true, 200, ArchiveOutcome::TimedOut)
            .unwrap();
        assert_eq!(reward.imprint_bonus, 0);
        assert_eq!(reward.reward, 60);
        assert_eq!(progress.currency, 60);
    }
    #[test]
    fn finished_run_settlement_reads_the_archive_outcome() {
        for (outcome, expected) in [
            (ArchiveOutcome::Captured, 80),
            (ArchiveOutcome::TimedOut, 65),
        ] {
            let mut app = App::new();
            app.init_resource::<CampaignProgress>()
                .init_resource::<CampaignReward>()
                .init_resource::<super::super::campaign::CampaignRun>()
                .init_resource::<ArchiveState>()
                .insert_resource(GameSession {
                    finished: true,
                    victory: true,
                    ..default()
                })
                .add_systems(Last, settle_finished_run);
            app.world_mut().resource_mut::<ArchiveState>().outcome = outcome;
            app.update();
            app.update();
            assert_eq!(
                app.world().resource::<CampaignProgress>().currency,
                expected
            );
        }
    }
    #[test]
    fn defeat_has_a_small_guaranteed_reward_without_unlocking() {
        let mut progress = CampaignProgress::default();
        let reward = CampaignReward::default()
            .settle(&mut progress, 0, false, 0, ArchiveOutcome::Pending)
            .unwrap();
        assert_eq!(reward.reward, 5);
        assert_eq!(reward.first_clear_bonus, 0);
        assert_eq!(reward.imprint_bonus, 0);
        assert_eq!(progress.unlocked, 1);
        assert!(!progress.cleared[0]);
        assert!(
            CampaignReward::default()
                .settle(&mut progress, 5, true, 999, ArchiveOutcome::Captured)
                .is_none()
        );
    }
    #[test]
    fn upgrade_costs_rise_stop_at_ten_and_cannot_overspend() {
        let mut progress = CampaignProgress::default();
        assert!(!progress.purchase(0));
        progress.currency = 100_000;
        let mut previous = 0;
        for level in 0..MAX_UPGRADE_LEVEL {
            let cost = progress.upgrade_cost(0).unwrap();
            assert!(cost > previous);
            let balance = progress.currency;
            assert!(progress.purchase(0));
            assert_eq!(progress.currency, balance - cost);
            assert_eq!(progress.upgrades[0], level + 1);
            previous = cost;
        }
        assert_eq!(progress.upgrade_cost(0), None);
        assert!(!progress.purchase(0));
        assert!(!progress.purchase(6));
    }
    #[test]
    fn all_six_chapters_roundtrip_and_corrupted_or_inconsistent_data_is_rejected() {
        let mut progress = CampaignProgress::default();
        for stage in 0..6 {
            assert!(
                CampaignReward::default()
                    .settle(&mut progress, stage, true, 50, ArchiveOutcome::TimedOut)
                    .is_some()
            );
        }
        assert_eq!(progress.unlocked, 6);
        progress.upgrades = [1, 2, 3, 4, 5, 6];
        let encoded = progress.encode();
        let decoded = CampaignProgress::decode(&encoded).unwrap();
        assert_eq!(decoded.upgrades, progress.upgrades);
        assert_eq!(decoded.currency, progress.currency);
        assert_eq!(decoded.cleared, [true; 6]);
        assert!(CampaignProgress::decode(&encoded.replace("1,2,3", "9,2,3")).is_err());
        progress.upgrades[0] = 11;
        assert!(CampaignProgress::decode(&progress.encode()).is_err());
        progress.upgrades[0] = 0;
        progress.cleared[2] = false;
        assert!(CampaignProgress::decode(&progress.encode()).is_err());
    }
    #[test]
    fn permanent_bonuses_apply_once_after_selection_and_survive_level_growth() {
        let mut app = App::new();
        app.init_resource::<CampaignProgress>()
            .init_resource::<AppliedPermanentBonus>()
            .init_resource::<SelectedHero>()
            .init_resource::<GameSession>()
            .init_resource::<PlayerBuild>()
            .init_resource::<HeroVitals>()
            .add_systems(
                Update,
                (super::super::hero::synchronize_hero_growth, apply_new_run).chain(),
            );
        app.world_mut().resource_mut::<CampaignProgress>().upgrades = [1; 6];
        let initial = app.world().resource::<HeroVitals>().strength;
        app.update();
        assert_eq!(
            app.world().resource::<HeroVitals>().strength,
            initial,
            "selection must precede permanent bonuses"
        );
        app.world_mut().resource_mut::<SelectedHero>().locked = true;
        app.update();
        let strength = app.world().resource::<HeroVitals>().strength;
        let attack = app.world().resource::<PlayerBuild>().attack_damage;
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(app.world().resource::<HeroVitals>().strength, strength);
        assert_eq!(app.world().resource::<PlayerBuild>().attack_damage, attack);
        app.world_mut().resource_mut::<GameSession>().level = 2;
        app.update();
        let vitals = app.world().resource::<HeroVitals>();
        let intrinsic = HeroVitals::for_hero(vitals.kind, 2);
        assert!((vitals.strength - intrinsic.strength - 2.).abs() < 0.001);
        assert!((vitals.spell_power() - intrinsic.spell_power() - 8.).abs() < 0.001);
        assert!((vitals.max_health - intrinsic.max_health - 58.).abs() < 0.001);
    }
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn isolated_atomic_file_roundtrip_retains_existing_corrupt_save() {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "rogue-campaign-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let path = dir.join("progress.save");
        let mut first = CampaignProgress::load_from_path(&path).unwrap();
        CampaignReward::default()
            .settle(&mut first, 0, true, 200, ArchiveOutcome::TimedOut)
            .unwrap();
        assert!(first.purchase(0));
        first.save().unwrap();
        let second = CampaignProgress::load_from_path(&path).unwrap();
        assert_eq!(second.upgrades[0], 1);
        assert_eq!(second.currency, 60);
        assert!(second.can_play(1));
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "atomic temporary file must not remain"
        );
        std::fs::write(&path, "broken save").unwrap();
        assert!(CampaignProgress::load_from_path(&path).is_err());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "broken save");
    }
}
