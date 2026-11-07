//! Renderer-free balance run using the complete production GamePlugin.
use bevy::{
    audio::AudioPlugin,
    ecs::schedule::{ScheduleLabel, SingleThreadedExecutor},
    prelude::*,
    render::RenderPlugin,
    time::TimeUpdateStrategy,
    window::ExitCondition,
    winit::WinitPlugin,
};
use rogue_commander::game::{
    BasicAttackCounter, BondCardId, BondDrawButton, CombatPools, DraftButton, DraftChoice,
    DraftState, EncounterState, Enemy, EnemyPressureSettings, GamePlugin, GameSession, Hero,
    HeroVitals, PlayerBuild, Projectile, RecruitmentProgress, RunRng, RunSeed, SelectedHero,
    SkillCastTelemetry, SkillDrawButton, SkillEconomy, SkillId, bond_cost, skill_cost,
};
use std::{env, time::Duration};

#[derive(Resource)]
struct Policy {
    chapter: usize,
    training: u8,
    active: bool,
    starter: usize,
    forward: bool,
    frame: u32,
    settled: u32,
    market_step: Option<usize>,
    next_market_at: f32,
    last_offer_root: Option<Entity>,
    pending_offer: Option<PendingOffer>,
}
struct PendingOffer {
    root: Entity,
    frame: u32,
    choice: DraftChoice,
    ranks: [u8; SkillId::COUNT],
    funds: [u32; 3],
}

/// Frozen ASCII identifiers keep this diagnostic independent of UI language.
fn choice_record(index: usize, choice: DraftChoice, build: &PlayerBuild) -> String {
    let rarity = choice.rarity().label();
    let fields = match choice {
        DraftChoice::Skill(skill) => format!(
            "\"kind\":\"skill\",\"skill\":{},\"base_rank\":{}",
            skill.0,
            build.skills[usize::from(skill.0)]
        ),
        DraftChoice::Evolution { skill, tier } => format!(
            "\"kind\":\"evolution\",\"skill\":{},\"base_rank\":{},\"tier\":{tier}",
            skill.0,
            build.skills[usize::from(skill.0)]
        ),
        DraftChoice::EvolveUr { skill, path } => format!(
            "\"kind\":\"ur_evolution\",\"skill\":{},\"base_rank\":{},\"path\":{}",
            skill.0,
            build.skills[usize::from(skill.0)],
            path.index()
        ),
        DraftChoice::ReplaceSkill { old, new } => format!(
            "\"kind\":\"replace_skill\",\"old\":{},\"new\":{},\"old_rarity\":\"{}\",\"new_rarity\":\"{}\",\"old_base_rank\":{},\"new_base_rank\":{}",
            old.0,
            new.0,
            old.definition().rarity.label(),
            new.definition().rarity.label(),
            build.skills[usize::from(old.0)],
            build.skills[usize::from(new.0)]
        ),
        DraftChoice::Bond(card) => format!(
            "\"kind\":\"bond\",\"card\":{},\"family\":{},\"family_rank\":{}",
            card.index(),
            card.family().index(),
            build.bond(card.family())
        ),
        DraftChoice::ReplaceBond(family) => format!(
            "\"kind\":\"replace_bond\",\"family\":{},\"family_rank\":{}",
            family.index(),
            build.bond(family)
        ),
        DraftChoice::Encounter(event) => {
            format!("\"kind\":\"encounter\",\"event\":{}", event.index())
        }
    };
    format!("{{\"index\":{index},\"rarity\":\"{rarity}\",{fields}}}")
}

fn record_offer(world: &mut World, frame: u32) {
    let Some(root) = world.resource::<DraftState>().root else {
        return;
    };
    if world.resource::<Policy>().last_offer_root == Some(root) {
        return;
    }
    let draft = world.resource::<DraftState>();
    let build = world.resource::<PlayerBuild>();
    let choices = draft
        .choices
        .iter()
        .enumerate()
        .map(|(index, choice)| choice_record(index, *choice, build))
        .collect::<Vec<_>>()
        .join(",");
    let session = world.resource::<GameSession>();
    let policy_frame = world.resource::<Policy>().frame;
    let root_bits = root.to_bits();
    println!(
        "{{\"type\":\"draft_offer\",\"frame\":{frame},\"policy_frame\":{policy_frame},\"elapsed\":{},\"root\":{root_bits},\"choices\":[{choices}],\"skill_ranks_by_id\":{:?},\"gold\":{},\"wood\":{},\"skill_points\":{}}}",
        session.elapsed, build.skills, session.gold, session.wood, session.skill_points
    );
    world.resource_mut::<Policy>().last_offer_root = Some(root);
}

/// Observe after app.update, never add a scheduling edge to gameplay systems.
fn record_settlement(world: &mut World, frame: u32) {
    if world.resource::<Policy>().pending_offer.is_none() {
        return;
    }
    let pending = world.resource_mut::<Policy>().pending_offer.take().unwrap();
    let build = world.resource::<PlayerBuild>();
    let retired = world.resource::<DraftState>().root != Some(pending.root);
    let replacement = if let DraftChoice::ReplaceSkill { old, new } = pending.choice {
        if retired {
            assert_eq!(
                build.skills[usize::from(old.0)],
                0,
                "settled replacement must retire the selected old skill"
            );
            assert_eq!(
                build.skills[usize::from(new.0)],
                1,
                "settled replacement must grant one new rank"
            );
        }
        format!(
            "{{\"old\":{},\"new\":{},\"old_base_rank_before\":{},\"new_base_rank_before\":{},\"old_base_rank_after\":{},\"new_base_rank_after\":{}}}",
            old.0,
            new.0,
            pending.ranks[usize::from(old.0)],
            pending.ranks[usize::from(new.0)],
            build.skills[usize::from(old.0)],
            build.skills[usize::from(new.0)]
        )
    } else {
        "null".to_owned()
    };
    let session = world.resource::<GameSession>();
    let root_bits = pending.root.to_bits();
    let request_policy_frame = pending.frame;
    let policy_frame = world.resource::<Policy>().frame;
    assert_eq!(
        request_policy_frame, policy_frame,
        "selection request must be observed in its same policy update"
    );
    println!(
        "{{\"type\":\"draft_settlement\",\"frame\":{frame},\"policy_frame\":{policy_frame},\"request_policy_frame\":{request_policy_frame},\"elapsed\":{},\"root\":{root_bits},\"selected_index\":0,\"root_retired\":{retired},\"replacement\":{replacement},\"ranks_before\":{:?},\"ranks_after\":{:?},\"funds_before\":{:?},\"funds_after\":[{},{},{}]}}",
        session.elapsed,
        pending.ranks,
        build.skills,
        pending.funds,
        session.gold,
        session.wood,
        session.skill_points
    );
}

fn press(world: &mut World, entity: Entity) {
    if let Some(mut interaction) = world.get_mut::<Interaction>(entity) {
        *interaction = Interaction::Pressed;
    }
}
fn policy(world: &mut World) {
    let (frame, active, starter, forward) = {
        let mut p = world.resource_mut::<Policy>();
        p.frame += 1;
        (p.frame, p.active, p.starter, p.forward)
    };
    world
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::KeyB);
    // Release between choices: Changed<Interaction> must represent a fresh input.
    let entities: Vec<Entity> = world
        .query::<(Entity, &Interaction)>()
        .iter(world)
        .filter_map(|(e, i)| (*i == Interaction::Pressed).then_some(e))
        .collect();
    for e in entities {
        *world.get_mut::<Interaction>(e).unwrap() = Interaction::None;
    }
    if frame == 1 {
        let requested = world.resource::<Policy>();
        let (chapter, training) = (requested.chapter, requested.training);
        let mut progress = world.resource_mut::<rogue_commander::game::CampaignProgress>();
        // Isolated chapter fixture, not a claim that this strategy earned prior clears.
        progress.unlocked = (chapter + 1) as u8;
        progress.cleared = std::array::from_fn(|index| index < chapter);
        progress.upgrades = [training; 6];
    }
    if frame < 10 || !frame.is_multiple_of(8) {
        return;
    }
    if world.resource::<rogue_commander::game::CampaignMenu>().0 {
        let wanted = format!("Campaign chapter {}", world.resource::<Policy>().chapter);
        let e = world
            .query::<(Entity, &Name)>()
            .iter(world)
            .find_map(|(e, n)| (n.as_str() == wanted).then_some(e));
        if let Some(e) = e {
            press(world, e);
        }
        return;
    }
    if !world.resource::<SelectedHero>().locked {
        let paths = [
            "ui/portraits/hero_stone_warden.png",
            "ui/portraits/hero_ember_ranger.png",
            "ui/portraits/hero_storm_savant.png",
        ];
        let wanted = world
            .resource::<AssetServer>()
            .load::<Image>(paths[starter])
            .id();
        let images: Vec<Entity> = world
            .query::<(Entity, &ImageNode)>()
            .iter(world)
            .filter_map(|(e, i)| (i.image.id() == wanted).then_some(e))
            .collect();
        for image in images {
            let mut card = Some(image);
            while let Some(e) = card {
                if world
                    .get::<Name>(e)
                    .is_some_and(|n| n.as_str().starts_with("Commander card "))
                {
                    press(world, e);
                    return;
                }
                card = world.get::<ChildOf>(e).map(ChildOf::parent);
            }
        }
        return;
    }
    if world.resource::<Policy>().settled == 0 {
        if forward {
            for mut t in world
                .query_filtered::<&mut Transform, With<Hero>>()
                .iter_mut(world)
            {
                t.translation.x = -2.0;
            }
        }
        world.resource_mut::<Policy>().settled = 1;
    }
    if world.resource::<GameSession>().finished {
        return;
    }
    // Real recruitment cards and real authoritative draft settlements; no stat grants.
    let hero_card = world
        .query::<(Entity, &Name, &Interaction)>()
        .iter(world)
        .find_map(|(e, n, _)| (n.as_str() == "Commander card 0").then_some(e));
    if let Some(e) = hero_card {
        press(world, e);
        return;
    }
    if !active {
        return;
    }
    if let Some(step) = world.resource::<Policy>().market_step {
        if step < 5 {
            let label = format!("Market offer {step}");
            let e = world
                .query::<(Entity, &Name)>()
                .iter(world)
                .find_map(|(e, n)| (n.as_str() == label).then_some(e));
            if let Some(e) = e {
                press(world, e);
            }
            world.resource_mut::<Policy>().market_step = Some(step + 1);
        } else {
            world
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyB);
            world.resource_mut::<Policy>().market_step = None;
        }
        return;
    }
    if world.resource::<DraftState>().root.is_some() {
        let e = world
            .query::<(Entity, &DraftButton)>()
            .iter(world)
            .find_map(|(e, b)| (b.0 == 0).then_some(e));
        if let Some(e) = e {
            let draft = world.resource::<DraftState>();
            let session = world.resource::<GameSession>();
            let pending = PendingOffer {
                root: draft.root.unwrap(),
                frame,
                choice: draft.choices[0],
                ranks: world.resource::<PlayerBuild>().skills,
                funds: [session.gold, session.wood, session.skill_points],
            };
            world.resource_mut::<Policy>().pending_offer = Some(pending);
            press(world, e);
        }
        return;
    }
    let s = world.resource::<GameSession>();
    if s.paused {
        return;
    }
    if s.elapsed >= world.resource::<Policy>().next_market_at && s.kills > 0 {
        let next = s.elapsed + 60.0;
        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyB);
        let mut p = world.resource_mut::<Policy>();
        p.market_step = Some(0);
        p.next_market_at = next;
        return;
    }
    let skill = s.skill_points > 0 || s.gold >= skill_cost(world.resource::<SkillEconomy>());
    let bond = s.wood >= bond_cost(s, world.resource::<EncounterState>());
    let e = if skill {
        world
            .query_filtered::<Entity, With<SkillDrawButton>>()
            .iter(world)
            .next()
    } else if bond {
        world
            .query_filtered::<Entity, With<BondDrawButton>>()
            .iter(world)
            .next()
    } else {
        None
    };
    if let Some(e) = e {
        press(world, e);
    }
}
/// Optional diagnostic output only; schedules and simulated time stay unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TraceRange {
    first: u32,
    last: u32,
}
impl TraceRange {
    fn parse(value: &str, frame_cap: u32) -> Result<Self, &'static str> {
        let (first, last) = value.split_once(':').ok_or("expected FIRST:LAST")?;
        let first = first.parse::<u32>().map_err(|_| "invalid first frame")?;
        let last = last.parse::<u32>().map_err(|_| "invalid last frame")?;
        if first == 0 || first > last || last > frame_cap {
            return Err("trace must satisfy 1 <= FIRST <= LAST <= frame cap");
        }
        Ok(Self { first, last })
    }
    fn contains(self, frame: u32) -> bool {
        (self.first..=self.last).contains(&frame)
    }
}

#[derive(Debug)]
struct TraceActors {
    hero_positions: Vec<[u64; 4]>,
    enemy_order: Vec<u64>,
    // Entity bits, health/max-health bits, position XYZ bits, speed bits.
    enemies: Vec<[u64; 7]>,
    // Entity bits, position XYZ, velocity XYZ, damage/radius/lifetime bits, hits.
    projectiles: Vec<[u64; 11]>,
}
fn trace_actors(world: &mut World) -> TraceActors {
    let mut hero_positions: Vec<_> = world
        .query_filtered::<(Entity, &Transform), With<Hero>>()
        .iter(world)
        .map(|(entity, transform)| {
            [
                entity.to_bits(),
                u64::from(transform.translation.x.to_bits()),
                u64::from(transform.translation.y.to_bits()),
                u64::from(transform.translation.z.to_bits()),
            ]
        })
        .collect();
    hero_positions.sort_unstable_by_key(|row| row[0]);
    let mut enemies: Vec<_> = world
        .query::<(Entity, &Enemy, &Transform)>()
        .iter(world)
        .map(|(entity, enemy, transform)| {
            [
                entity.to_bits(),
                u64::from(enemy.hp.to_bits()),
                u64::from(enemy.max_hp.to_bits()),
                u64::from(transform.translation.x.to_bits()),
                u64::from(transform.translation.y.to_bits()),
                u64::from(transform.translation.z.to_bits()),
                u64::from(enemy.speed.to_bits()),
            ]
        })
        .collect();
    let enemy_order = enemies.iter().map(|row| row[0]).collect();
    enemies.sort_unstable_by_key(|row| row[0]);
    let mut projectiles: Vec<_> = world
        .query::<(Entity, &Projectile, &Transform)>()
        .iter(world)
        .map(|(entity, projectile, transform)| {
            [
                entity.to_bits(),
                u64::from(transform.translation.x.to_bits()),
                u64::from(transform.translation.y.to_bits()),
                u64::from(transform.translation.z.to_bits()),
                u64::from(projectile.velocity.x.to_bits()),
                u64::from(projectile.velocity.y.to_bits()),
                u64::from(projectile.velocity.z.to_bits()),
                u64::from(projectile.damage.to_bits()),
                u64::from(projectile.radius.to_bits()),
                u64::from(projectile.lifetime.to_bits()),
                u64::from(projectile.remaining_hits),
            ]
        })
        .collect();
    projectiles.sort_unstable_by_key(|row| row[0]);
    TraceActors {
        hero_positions,
        enemy_order,
        enemies,
        projectiles,
    }
}
fn trace_update(world: &mut World, frame: u32) {
    let actors = trace_actors(world);
    let session = world.resource::<GameSession>();
    let hero = world.resource::<HeroVitals>();
    let build = world.resource::<PlayerBuild>();
    let rng = world.resource::<RunRng>().state();
    let attacks = world.resource::<BasicAttackCounter>().count();
    let casts = world.resource::<SkillCastTelemetry>().total;
    let hero_bits = [
        hero.health,
        hero.max_health,
        hero.energy,
        hero.max_energy,
        hero.strength,
        hero.agility,
        hero.intelligence,
        hero.revive_remaining,
        hero.invulnerable_remaining,
        hero.stunned_remaining,
        build.attack_damage,
        build.attacks_per_second,
    ]
    .map(f32::to_bits);
    let owned_bonds: Vec<_> = BondCardId::ALL
        .iter()
        .filter(|id| build.owned_bond_cards[id.index()])
        .map(|id| id.index())
        .collect();
    let economy = world.resource::<SkillEconomy>();
    println!(
        "{{\"type\":\"trace\",\"frame\":{frame},\"elapsed_bits\":{},\"gate_bits\":{},\"gate_revive_bits\":{},\"rng_state\":{rng},\"attacks\":{attacks},\"casts\":{casts},\"hero_id\":{},\"hero_bits\":{hero_bits:?},\"hero_alive\":{},\"deaths\":{},\"level\":{},\"xp\":{},\"gold\":{},\"wood\":{},\"kills\":{},\"skill_points\":{},\"paid_skill_draws\":{},\"paid_bond_draws\":{},\"skill_ranks\":{:?},\"owned_bonds\":{owned_bonds:?},\"paused\":{},\"finished\":{},\"hero_positions\":{:?},\"enemy_order\":{:?},\"enemies\":{:?},\"projectiles\":{:?}}}",
        session.elapsed.to_bits(),
        session.core_hp.to_bits(),
        session.gate_revive_remaining.to_bits(),
        hero.kind.index(),
        hero.alive,
        hero.deaths,
        session.level,
        session.xp,
        session.gold,
        session.wood,
        session.total_kills,
        session.skill_points,
        economy.paid_draws,
        session.bond_draws,
        build.skills,
        session.paused,
        session.finished,
        actors.hero_positions,
        actors.enemy_order,
        actors.enemies,
        actors.projectiles,
    );
}

struct Metrics {
    gate_breaks: u32,
    gate_down: bool,
    exposed_secs: f32,
    last_elapsed: f32,
    min_gate_hp: f32,
    min_hero_hp: f32,
    min_alive_health_fraction: f32,
    max_health_deficit: f32,
    injured_secs: f32,
}
impl Metrics {
    fn observe(&mut self, world: &World) {
        let s = world.resource::<GameSession>();
        let h = world.resource::<HeroVitals>();
        let down = s.core_hp <= 0.0 || s.gate_revive_remaining > 0.0;
        if down && !self.gate_down {
            self.gate_breaks += 1;
        }
        let seconds = (s.elapsed - self.last_elapsed).max(0.0);
        if down {
            self.exposed_secs += seconds;
        }
        if h.alive && h.max_health > 0.0 {
            self.min_alive_health_fraction = self
                .min_alive_health_fraction
                .min((h.health / h.max_health).clamp(0.0, 1.0));
            let deficit = (h.max_health - h.health).max(0.0);
            self.max_health_deficit = self.max_health_deficit.max(deficit);
            if deficit > 0.01 {
                self.injured_secs += seconds;
            }
        }
        self.gate_down = down;
        self.last_elapsed = s.elapsed;
        self.min_gate_hp = self.min_gate_hp.min(s.core_hp);
        self.min_hero_hp = self.min_hero_hp.min(h.health);
    }
}
fn checkpoint(world: &mut World, frame: u32, metrics: &Metrics) {
    let s = world.resource::<GameSession>();
    let h = world.resource::<HeroVitals>();
    let actual_chapter = world.resource::<rogue_commander::game::CampaignRun>().stage;
    let permanent_upgrades = world
        .resource::<rogue_commander::game::CampaignProgress>()
        .upgrades;
    let actual_seed = world.resource::<RunSeed>().0;
    let rng_state = world.resource::<RunRng>().0;
    let skill_ranks_by_id = world.resource::<PlayerBuild>().skills;
    let actual_hero = h.kind.index();
    let absorbed_strength = world.resource::<RecruitmentProgress>().absorbed_strength;
    let command_health_multiplier = h.command_health_multiplier;
    let max_hero_hp = h.max_health;
    let owned_bond_ids = BondCardId::ALL
        .iter()
        .filter(|id| world.resource::<PlayerBuild>().owned_bond_cards[id.index()])
        .map(|id| id.index().to_string())
        .collect::<Vec<_>>()
        .join(",");
    let attacks = world
        .get_resource::<BasicAttackCounter>()
        .map_or(0, |v| v.count());
    let casts = world
        .get_resource::<SkillCastTelemetry>()
        .map_or("null".to_owned(), |v| v.total.to_string());
    let skills: u32 = world
        .resource::<PlayerBuild>()
        .skills
        .iter()
        .map(|v| u32::from(*v))
        .sum();
    let pools = world.resource::<CombatPools>().snapshot();
    let gate_breaks = metrics.gate_breaks;
    let exposed_secs = metrics.exposed_secs;
    let min_gate_hp = metrics.min_gate_hp;
    let min_hero_hp = metrics.min_hero_hp;
    let min_alive_health_fraction = metrics.min_alive_health_fraction;
    let max_health_deficit = metrics.max_health_deficit;
    let injured_secs = metrics.injured_secs;
    println!(
        "{{\"type\":\"checkpoint\",\"frame\":{frame},\"actual_chapter\":{actual_chapter},\"permanent_upgrades\":{permanent_upgrades:?},\"actual_run_seed\":{actual_seed},\"rng_state\":{rng_state},\"skill_ranks_by_id\":{skill_ranks_by_id:?},\"actual_hero_id\":{actual_hero},\"owned_bond_ids\":[{owned_bond_ids}],\"absorbed_strength\":{absorbed_strength},\"command_health_multiplier\":{command_health_multiplier},\"max_hero_hp\":{max_hero_hp},\"elapsed\":{:.3},\"wave\":{},\"level\":{},\"kills\":{},\"gate_hp\":{:.2},\"gate_breaks\":{gate_breaks},\"exposed_secs\":{exposed_secs:.3},\"min_gate_hp\":{min_gate_hp:.2},\"min_hero_hp\":{min_hero_hp:.2},\"min_alive_health_fraction\":{min_alive_health_fraction:.6},\"max_health_deficit\":{max_health_deficit:.3},\"injured_secs\":{injured_secs:.3},\"hero_hp\":{:.2},\"deaths\":{},\"gold\":{},\"wood\":{},\"bosses\":{},\"attacks\":{attacks},\"damage_skill_casts\":{casts},\"skill_ranks\":{skills},\"enemy_peak\":{},\"enemy_rejected\":{},\"paused\":{},\"finished\":{},\"victory\":{}}}",
        s.elapsed,
        s.wave,
        s.level,
        s.total_kills,
        s.core_hp,
        h.health,
        h.deaths,
        s.gold,
        s.wood,
        s.bosses_defeated,
        pools.enemies.peak(),
        pools.enemies.rejected(),
        s.paused,
        s.finished,
        s.victory
    );
}
fn main() {
    let args: Vec<String> = env::args().collect();
    let chapter: usize = env::var("ROGUE_SIM_CHAPTER")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .expect("chapter must be an integer");
    let training: u8 = env::var("ROGUE_SIM_TRAINING")
        .unwrap_or_else(|_| "0".into())
        .parse()
        .expect("training must be an integer");
    assert!(
        chapter < rogue_commander::game::CHAPTER_COUNT,
        "invalid chapter"
    );
    assert!(
        training <= rogue_commander::game::MAX_UPGRADE_LEVEL,
        "invalid training"
    );
    let active = args.get(1).is_some_and(|s| s == "active");
    let seed = u64::from_str_radix(args.get(2).map_or("1", |s| s.trim_start_matches("0x")), 16)
        .expect("hex seed");
    let starter: usize = args.get(3).map_or("2", String::as_str).parse().unwrap();
    assert!(starter < 3, "invalid starter");
    let forward = args.get(4).is_some_and(|s| s == "forward");
    let frames: u32 = args.get(5).map_or("90000", String::as_str).parse().unwrap();
    assert!(
        args.len() <= 6 || args.len() == 12,
        "pressure override requires exactly six values"
    );
    let mut pressure = EnemyPressureSettings::default();
    if let Some(v) = args.get(6) {
        pressure.movement_growth = v.parse().expect("movement growth");
    }
    if let Some(v) = args.get(7) {
        pressure.attack_growth = v.parse().expect("attack growth");
    }
    if let Some(v) = args.get(8) {
        pressure.movement_cap = v.parse().expect("movement cap");
    }
    if let Some(v) = args.get(9) {
        pressure.attack_cap = v.parse().expect("attack cap");
    }
    if let Some(v) = args.get(10) {
        pressure.health_growth = v.parse().expect("health growth");
    }
    if let Some(v) = args.get(11) {
        pressure.health_cap = v.parse().expect("health cap");
    }
    let trace_range = env::var("ROGUE_SIM_TRACE_RANGE")
        .ok()
        .map(|value| TraceRange::parse(&value, frames).expect("invalid ROGUE_SIM_TRACE_RANGE"));
    let asset_root = env::var("ROGUE_SIM_ASSET_ROOT")
        .unwrap_or_else(|_| format!("{}/assets", env!("CARGO_MANIFEST_DIR")));
    assert!(
        std::path::Path::new(&asset_root).is_dir(),
        "simulation asset root is not a directory: {asset_root}"
    );
    println!("BALANCE_ASSET_ROOT {asset_root}");
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: asset_root,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (1280, 720).into(),
                    ..default()
                }),
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .disable::<RenderPlugin>()
            .disable::<WinitPlugin>()
            .disable::<AudioPlugin>()
            .disable::<bevy::core_pipeline::CorePipelinePlugin>()
            .disable::<bevy::post_process::PostProcessPlugin>()
            .disable::<bevy::anti_alias::AntiAliasPlugin>()
            .disable::<bevy::sprite_render::SpriteRenderPlugin>()
            .disable::<bevy::ui_render::UiRenderPlugin>()
            .disable::<bevy::pbr::PbrPlugin>()
            .disable::<bevy::gizmos_render::GizmoRenderPlugin>()
            .disable::<bevy::render::pipelined_rendering::PipelinedRenderingPlugin>(),
    )
    .init_asset::<StandardMaterial>()
    .init_asset::<bevy::audio::AudioSource>()
    .init_asset_loader::<bevy::audio::AudioLoader>()
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
        1.0 / 60.0,
    )))
    .insert_resource(pressure)
    .insert_resource(RunSeed(seed))
    .insert_resource(RunRng::from_seed(seed))
    .insert_resource(Policy {
        chapter,
        training,
        active,
        starter,
        forward,
        frame: 0,
        settled: 0,
        market_step: None,
        next_market_at: 180.0,
        last_offer_root: None,
        pending_offer: None,
    })
    .add_plugins(GamePlugin)
    .add_systems(PreUpdate, policy.after(bevy::ui::UiSystems::Focus));
    // Keep the complete production systems, but make their conflict ordering
    // repeatable. Presentation hit-stop also reads the same fixed clock.
    for label in [
        First.intern(),
        PreUpdate.intern(),
        Update.intern(),
        PostUpdate.intern(),
        Last.intern(),
    ] {
        app.edit_schedule(label, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
    }
    app.finish();
    app.cleanup();
    println!(
        "{{\"type\":\"metadata\",\"policy\":\"{}\",\"seed\":{seed},\"chapter\":{chapter},\"training\":{training},\"profile_fixture\":\"prior-chapters-cleared-six-equal-upgrades\",\"starter\":{starter},\"forward\":{forward},\"frame_cap\":{frames},\"renderer\":false,\"executor\":\"single-threaded\",\"fixed_step_secs\":0.016666666666666666}}",
        if active {
            "greedy-card-zero"
        } else {
            "passive"
        }
    );
    println!(
        "{{\"type\":\"pressure\",\"movement_growth\":{},\"attack_growth\":{},\"movement_cap\":{},\"attack_cap\":{},\"health_growth\":{},\"health_cap\":{}}}",
        pressure.movement_growth,
        pressure.attack_growth,
        pressure.movement_cap,
        pressure.attack_cap,
        pressure.health_growth,
        pressure.health_cap
    );
    let mut metrics = Metrics {
        gate_breaks: 0,
        gate_down: false,
        exposed_secs: 0.0,
        last_elapsed: 0.0,
        min_gate_hp: f32::MAX,
        min_hero_hp: f32::MAX,
        min_alive_health_fraction: 1.0,
        max_health_deficit: 0.0,
        injured_secs: 0.0,
    };
    for frame in 1..=frames {
        app.update();
        record_settlement(app.world_mut(), frame);
        record_offer(app.world_mut(), frame);
        metrics.observe(app.world());
        if trace_range.is_some_and(|range| range.contains(frame)) {
            trace_update(app.world_mut(), frame);
        }
        if frame == 32
            || frame.is_multiple_of(1800)
            || frame == frames
            || app.world().resource::<GameSession>().finished
        {
            checkpoint(app.world_mut(), frame, &metrics);
        }
        if app.world().resource::<GameSession>().finished {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_range_rejects_unbounded_or_invalid_payloads() {
        let range = TraceRange::parse("19000:32400", 120000).unwrap();
        assert!(!range.contains(18999));
        assert!(range.contains(19000) && range.contains(32400));
        assert!(!range.contains(32401));
        for invalid in [
            "",
            "19000",
            "0:10",
            "20:10",
            "1:120001",
            "1:2:3",
            "1:NaN",
            "1:4294967296",
        ] {
            assert!(TraceRange::parse(invalid, 120000).is_err(), "{invalid}");
        }
    }

    #[test]
    fn trace_actor_payload_keeps_exact_bits_and_separates_raw_order_from_canonical_state() {
        #[derive(Component)]
        struct Cosmetic;
        fn world(reverse: bool) -> World {
            let mut world = World::new();
            let actors: [Entity; 2] = std::array::from_fn(|_| {
                world
                    .spawn((
                        Enemy {
                            hp: 1.000_000_1,
                            max_hp: 100.,
                            speed: 0.25,
                            damage: 0.,
                            xp: 0,
                            wood: 0,
                            gold: 0,
                            kill_credit: 0,
                            elite: false,
                        },
                        Transform::from_xyz(-0., 0., 2.),
                    ))
                    .id()
            });
            world.spawn((
                Projectile {
                    velocity: Vec3::X,
                    damage: 7.25,
                    radius: 0.5,
                    remaining_hits: 2,
                    lifetime: 1.5,
                },
                Transform::default(),
            ));
            for entity in if reverse {
                [actors[1], actors[0]]
            } else {
                actors
            } {
                world.entity_mut(entity).insert(Cosmetic);
            }
            world
        }
        let forward = trace_actors(&mut world(false));
        let reverse = trace_actors(&mut world(true));
        assert_ne!(forward.enemy_order, reverse.enemy_order);
        assert_eq!(forward.enemies, reverse.enemies);
        assert_eq!(forward.projectiles, reverse.projectiles);
        assert_eq!(forward.hero_positions, reverse.hero_positions);
        assert!(
            forward
                .enemies
                .windows(2)
                .all(|pair| pair[0][0] < pair[1][0])
        );
        assert!(
            forward
                .enemies
                .iter()
                .all(|row| row[1] == u64::from(1.000_000_1_f32.to_bits())
                    && row[3] == u64::from((-0.0_f32).to_bits()))
        );
        assert_eq!(forward.projectiles[0][7], u64::from(7.25_f32.to_bits()));
    }

    fn policy_fixture(
        gold: u32,
        wood: u32,
        bond_draws: u32,
        skill_points: u32,
    ) -> (World, Entity, Entity) {
        let mut world = World::new();
        world.insert_resource(Policy {
            chapter: 5,
            training: 0,
            active: true,
            starter: 2,
            forward: false,
            frame: 15,
            settled: 1,
            market_step: None,
            next_market_at: 180.,
            last_offer_root: None,
            pending_offer: None,
        });
        world.init_resource::<ButtonInput<KeyCode>>();
        world.init_resource::<rogue_commander::game::CampaignMenu>();
        world
            .resource_mut::<rogue_commander::game::CampaignMenu>()
            .0 = false;
        world.init_resource::<SelectedHero>();
        world.resource_mut::<SelectedHero>().locked = true;
        world.init_resource::<GameSession>();
        {
            let mut session = world.resource_mut::<GameSession>();
            session.elapsed = 1.;
            session.gold = gold;
            session.wood = wood;
            session.bond_draws = bond_draws;
            session.skill_points = skill_points;
            session.paused = false;
        }
        world.init_resource::<DraftState>();
        world.init_resource::<SkillEconomy>();
        world.resource_mut::<SkillEconomy>().paid_draws = 8;
        world.init_resource::<rogue_commander::game::EncounterState>();
        let skill = world.spawn((SkillDrawButton, Interaction::None)).id();
        let bond = world.spawn((BondDrawButton, Interaction::None)).id();
        (world, skill, bond)
    }

    #[test]
    fn policy_chooses_affordable_bond_when_late_skill_draw_is_unaffordable() {
        let (mut world, skill, bond) = policy_fixture(260, 40, 0, 0);
        policy(&mut world);
        assert_eq!(world.get::<Interaction>(skill), Some(&Interaction::None));
        assert_eq!(world.get::<Interaction>(bond), Some(&Interaction::Pressed));
    }

    #[test]
    fn policy_does_not_request_bond_when_current_wood_price_exceeds_balance() {
        let (mut world, skill, bond) = policy_fixture(0, 40, 10, 0);
        policy(&mut world);
        assert_eq!(world.get::<Interaction>(skill), Some(&Interaction::None));
        assert_eq!(world.get::<Interaction>(bond), Some(&Interaction::None));
    }

    #[test]
    fn policy_prioritizes_free_skill_points_without_gold_or_wood() {
        let (mut world, skill, bond) = policy_fixture(0, 0, 10, 1);
        policy(&mut world);
        assert_eq!(world.get::<Interaction>(skill), Some(&Interaction::Pressed));
        assert_eq!(world.get::<Interaction>(bond), Some(&Interaction::None));
    }

    #[test]
    fn health_pressure_tracks_fraction_after_growth_and_excludes_dead_time() {
        let mut world = World::new();
        world.insert_resource(GameSession::default());
        world.insert_resource(HeroVitals::default());
        let mut metrics = Metrics {
            gate_breaks: 0,
            gate_down: false,
            exposed_secs: 0.0,
            last_elapsed: 0.0,
            min_gate_hp: f32::MAX,
            min_hero_hp: f32::MAX,
            min_alive_health_fraction: 1.0,
            max_health_deficit: 0.0,
            injured_secs: 0.0,
        };
        metrics.observe(&world);
        world.resource_mut::<GameSession>().elapsed = 2.0;
        {
            let mut h = world.resource_mut::<HeroVitals>();
            h.max_health = 10000.0;
            h.health = 2500.0;
        }
        metrics.observe(&world);
        assert_eq!(metrics.min_alive_health_fraction, 0.25);
        assert_eq!(metrics.max_health_deficit, 7500.0);
        assert_eq!(metrics.injured_secs, 2.0);
        world.resource_mut::<GameSession>().elapsed = 5.0;
        {
            let mut h = world.resource_mut::<HeroVitals>();
            h.alive = false;
            h.health = 0.0;
        }
        metrics.observe(&world);
        assert_eq!(metrics.min_alive_health_fraction, 0.25);
        assert_eq!(metrics.injured_secs, 2.0);
        assert_eq!(metrics.min_hero_hp, 0.0);
    }
}
