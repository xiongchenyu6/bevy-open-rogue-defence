use std::{collections::BTreeSet, env, fs, path::PathBuf, time::Duration};

use bevy::{
    app::SubApps,
    asset::RenderAssetUsages,
    image::Image,
    prelude::*,
    render::{
        RenderPlugin,
        render_resource::{Extent3d, PollType, TextureDimension, TextureFormat, TextureUsages},
        renderer::RenderDevice,
        view::screenshot::{Screenshot, ScreenshotCaptured, save_to_disk},
    },
    time::{TimeReceiver, TimeUpdateStrategy},
    window::ExitCondition,
    winit::WinitPlugin,
};
use rogue_commander::game::{
    ArchiveState, Autoplay, BondDrawButton, BondId, CampaignMenu, CaptureTarget, CombatPools,
    DraftRoot, DraftState, Enemy, GameAssets, GameLanguage, GamePlugin, GameSession,
    GoldChallengeState, PlayerBuild, Projectile, RunRng, RunSeed, SelectedHero, SkillDrawButton,
    SpawnDirector, TrialState, parse_run_seed,
};

#[path = "capture/immediate_choice.rs"]
mod immediate_choice_capture;

#[path = "capture/balance_cadence.rs"]
mod balance_cadence;
#[path = "capture/class_attacks.rs"]
mod class_attacks_capture;

#[path = "capture/adventure_boss.rs"]
mod adventure_boss_capture;

#[path = "capture/selection_detail.rs"]
mod selection_detail_capture;

#[path = "capture/gate_revival.rs"]
mod gate_revival_capture;

#[path = "capture/skill_identity.rs"]
mod skill_identity_capture;

#[path = "capture/ur_evolution.rs"]
mod ur_evolution_capture;

#[path = "capture/picker_settings.rs"]
mod picker_settings_capture;

#[path = "capture/onboarding.rs"]
mod onboarding_capture;

#[path = "capture/asset_readiness.rs"]
mod asset_readiness_capture;

#[path = "capture/campaign_save_retry.rs"]
mod campaign_save_retry_capture;

#[path = "capture/campaign.rs"]
mod campaign_capture;

#[path = "capture/tidal_wave.rs"]
mod tidal_wave_capture;

#[path = "capture/cultivation_combat.rs"]
mod cultivation_combat_capture;

#[path = "capture/bond_collection.rs"]
mod bond_collection_capture;
#[path = "capture/spirit_gallery.rs"]
mod spirit_gallery_capture;

#[path = "capture/summon_assault.rs"]
mod summon_assault_capture;

#[path = "capture/fusion.rs"]
mod fusion_capture;

#[path = "capture/growth.rs"]
mod growth_capture;

#[path = "capture/hud_input.rs"]
mod hud_input_capture;

#[path = "capture/enemy_attacks.rs"]
mod enemy_attacks_capture;

#[path = "capture/orbit_blade.rs"]
mod orbit_blade_capture;

#[path = "capture/skill_growth.rs"]
mod skill_growth_capture;

#[path = "capture/attacks.rs"]
mod attack_capture;

#[path = "capture/ultimates.rs"]
mod ultimate_capture;

#[path = "capture/skill_spectacle.rs"]
mod skill_spectacle_capture;

#[path = "capture/audio_settings.rs"]
mod audio_settings_capture;

#[path = "capture/text_idle.rs"]
mod text_idle_capture;

#[path = "capture/settings_navigation.rs"]
mod settings_navigation_capture;

#[path = "capture/settings.rs"]
mod settings_capture;

#[path = "capture/trials.rs"]
mod trial_capture;

#[path = "capture/recruitment.rs"]
mod recruitment_capture;

#[path = "capture/bonds.rs"]
mod bond_capture;

#[path = "capture/bond_completion.rs"]
mod bond_completion_capture;

#[path = "capture/demon_cave.rs"]
mod demon_cave_capture;

#[path = "capture/elite_pressure.rs"]
mod elite_pressure_capture;
#[path = "capture/raid_pressure.rs"]
mod raid_pressure_capture;

#[path = "capture/boss_battle.rs"]
mod boss_battle_capture;

#[path = "capture/crafting.rs"]
mod crafting_capture;

#[path = "capture/equipment.rs"]
mod equipment_capture;

#[path = "capture/attribute_growth.rs"]
mod attribute_growth_capture;

#[path = "capture/hero_attributes.rs"]
mod hero_attributes_capture;

#[path = "capture/bond_opening.rs"]
mod bond_opening_capture;

#[derive(Resource, Default)]
struct CaptureControls {
    suppress_drafts: bool,
}

#[derive(Resource)]
struct DraftCapture {
    scenario: String,
    frame: u32,
}

fn exercise_commander_draft(
    mut capture: ResMut<DraftCapture>,
    mut buttons: Query<(&Name, &mut Interaction), With<Button>>,
    selected: Res<SelectedHero>,
    session: Res<GameSession>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>,
    windows: Query<&Window>,
) {
    capture.frame += 1;
    let target = match (capture.scenario.as_str(), capture.frame) {
        ("draft-preview" | "draft-confirm", 30) => Some("Commander card 2"),
        _ => None,
    };
    if let Some(target) = target {
        let mut found = false;
        for (name, mut interaction) in &mut buttons {
            if name.as_str() == target {
                *interaction = if capture.scenario == "draft-preview" {
                    Interaction::Hovered
                } else {
                    Interaction::Pressed
                };
                found = true;
            }
        }
        assert!(found, "missing capture control: {target}");
        println!("[capture] pressed {target}");
    }
    if capture.frame == 50 && capture.scenario == "draft-preview" {
        assert!(
            !selected.locked && session.paused,
            "preview must not start battle"
        );
        let window = windows.single().expect("capture window");
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        let mut cards = 0;
        for (name, node, transform) in &layout {
            if !name.as_str().starts_with("Commander ") {
                continue;
            }
            let half = node.size() * 0.5;
            let min = transform.translation - half;
            let max = transform.translation + half;
            assert!(
                min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
                "{name} outside viewport: {min:?}..{max:?}"
            );
            assert!(
                node.size().min_element() >= 44.0,
                "{name} too small for touch"
            );
            cards += usize::from(name.as_str().starts_with("Commander card "));
        }
        assert_eq!(cards, 3, "all basic commanders must be visible");
        println!("[capture] draft preview keeps battle paused and hero uncommitted");
        println!("[capture] all three cards and actions fit viewport with touch targets >=44px");
    }
    if capture.frame == 90 && capture.scenario == "draft-confirm" {
        assert!(
            selected.locked && !session.paused,
            "confirmation must start battle"
        );
        println!("[capture] confirmation committed hero and resumed battle");
    }
}

fn show_bond_hint(mut bonds: Query<&mut Interaction, With<BondDrawButton>>) {
    for mut interaction in &mut bonds {
        *interaction = Interaction::Hovered;
    }
}

#[derive(Resource)]
struct BuildCapture {
    scenario: String,
    frame: u32,
}

#[allow(clippy::type_complexity)] // Bevy query combines capture names and draw-button markers.
fn exercise_build_draft(
    mut capture: ResMut<BuildCapture>,
    mut buttons: Query<
        (
            Option<&Name>,
            &mut Interaction,
            Has<BondDrawButton>,
            Has<SkillDrawButton>,
        ),
        With<Button>,
    >,
    draft: Res<DraftState>,
    session: Res<GameSession>,
    layout: Query<(&Name, &ComputedNode, &UiGlobalTransform), With<Button>>,
    windows: Query<&Window>,
) {
    capture.frame += 1;
    let target = match (capture.scenario.as_str(), capture.frame) {
        ("build-confirm", 65) => Some("Build card 1"),
        ("build-refresh", 65) => Some("Build refresh"),
        ("build-later", 65) => Some("Build later"),
        ("build-abandon", 65) => Some("Build abandon"),
        _ => None,
    };
    let mut pressed = false;
    for (name, mut interaction, bond, skill) in &mut buttons {
        if capture.frame >= 100 && capture.scenario.starts_with("build-hover-") {
            let target = if capture.scenario == "build-hover-action" {
                "Build abandon"
            } else {
                "Build card 0"
            };
            *interaction = if name.is_some_and(|name| name.as_str() == target) {
                Interaction::Hovered
            } else {
                Interaction::None
            };
        }
        let request = capture.frame == 5
            && if capture.scenario == "build-skill" {
                skill
            } else {
                bond
            };
        if request || target.is_some_and(|target| name.is_some_and(|name| name.as_str() == target))
        {
            *interaction = Interaction::Pressed;
            pressed = true;
        }
    }
    if let Some(target) = target {
        assert!(pressed, "missing build control: {target}");
        println!("[capture] pressed {target}");
    }
    if capture.frame == 50 {
        assert!(
            session.paused && draft.root.is_some(),
            "hover keeps draft open"
        );
        let window = windows.single().unwrap();
        let viewport = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        let mut count = 0;
        for (name, node, transform) in &layout {
            if !name.as_str().starts_with("Build ") || node.size().min_element() == 0.0 {
                continue;
            }
            let half = node.size() * 0.5;
            let min = transform.translation - half;
            let max = transform.translation + half;
            assert!(
                min.cmpge(Vec2::splat(-1.0)).all() && max.cmple(viewport + Vec2::ONE).all(),
                "{name} outside viewport: {min:?}..{max:?}"
            );
            assert!(
                node.size().min_element() >= 44.0,
                "{name} too small for touch"
            );
            count += usize::from(name.as_str().starts_with("Build card "));
        }
        assert_eq!(count, draft.choices.len());
        println!("[capture] build preview paused; all cards/actions fit viewport");
    }
    if capture.frame == 90
        && matches!(
            capture.scenario.as_str(),
            "build-confirm" | "build-later" | "build-abandon"
        )
    {
        assert!(
            !session.paused && draft.root.is_none(),
            "build action must close overlay"
        );
        println!("[capture] build action closed overlay and resumed battle");
    }
}

#[derive(Resource, Default)]
struct CaptureSaveStatus {
    expected: u32,
    saved: u32,
    failed: u32,
}

const CAPTURE_WAIT_FRAMES: u32 = 120;
const SIMULATE_RENDER_SETTLE_FRAMES: u32 = 4;
const SIMULATE_BOOTSTRAP_RENDER_FRAMES: u32 = 8;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args
        .get(1)
        .is_some_and(|arg| matches!(arg.as_str(), "--help" | "-h"))
    {
        println!(
            "capture [still|frames|simulate] OUTPUT FRAMES SCENARIO [SEED] [WIDTHxHEIGHT]\n\nROGUE_CAPTURE_SAVE_FRAMES=0,25,99 selects zero-based PNG frames in frames mode only.\nMissing or empty saves every frame. All simulation/render updates still run."
        );
        return;
    }
    let mode = args.get(1).map(String::as_str).unwrap_or("still");
    let output = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("screenshots/result.png"));
    let frame_count = args
        .get(3)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(180);
    let selected_frames = if mode == "frames" {
        env::var("ROGUE_CAPTURE_SAVE_FRAMES").ok().filter(|value| !value.trim().is_empty()).map(|value| {
            let frames: BTreeSet<u32> = value.split(',').map(|entry| {
                let frame = entry.trim().parse::<u32>().unwrap_or_else(|_| {
                    panic!("ROGUE_CAPTURE_SAVE_FRAMES: invalid frame {entry:?}; expected comma-separated nonnegative integers")
                });
                assert!(frame < frame_count, "ROGUE_CAPTURE_SAVE_FRAMES: frame {frame} out of range; expected 0..{frame_count} (end exclusive)");
                frame
            }).collect();
            println!("[capture] selective PNG frames: {frames:?}; all {frame_count} simulation/render updates retained");
            frames
        })
    } else {
        None
    };
    let scenario = args.get(4).map(String::as_str).unwrap_or("standard");
    let requested_seed = args
        .get(5)
        .map(String::as_str)
        .map(|value| parse_run_seed(value).unwrap_or_else(|| panic!("invalid run seed: {value}")));

    if mode == "frames" {
        fs::create_dir_all(&output).expect("create capture directory");
    } else if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).expect("create screenshot directory");
    }

    let (width, height) = args.get(6).map_or((1280, 720), |size| {
        let (width, height) = size
            .split_once('x')
            .expect("capture size must be WIDTHxHEIGHT");
        let width = width.parse::<u32>().expect("invalid capture width");
        let height = height.parse::<u32>().expect("invalid capture height");
        assert!((320..=3840).contains(&width) && (240..=2160).contains(&height));
        (width, height)
    });

    let mut app = App::new();
    if scenario == "asset-readiness" {
        asset_readiness_capture::prepare(&mut app);
    }
    app.insert_resource(ClearColor(Color::srgb(0.015, 0.025, 0.045)))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
            1.0 / 30.0,
        )))
        .insert_resource(Autoplay)
        .init_resource::<CaptureControls>()
        .init_resource::<CaptureSaveStatus>()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: format!("{}/assets", env!("CARGO_MANIFEST_DIR")),
                    ..default()
                })
                .set(WindowPlugin {
                    // A logical window supplies the same responsive HUD measurements
                    // as native/Web play. Winit stays disabled: rendering is offscreen.
                    primary_window: Some(Window {
                        resolution: (width, height).into(),
                        ..default()
                    }),
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                })
                .disable::<WinitPlugin>(),
        )
        .add_plugins(GamePlugin)
        .add_systems(PostUpdate, suppress_final_drafts);

    if scenario == "asset-readiness" {
        asset_readiness_capture::plugin(&mut app);
    }

    if scenario.starts_with("onboarding-") {
        onboarding_capture::plugins(&mut app);
    }
    if let Some(seed) = requested_seed {
        app.insert_resource(RunSeed(seed));
        app.insert_resource(RunRng::from_seed(seed));
    }
    if let Ok(language) = env::var("ROGUE_COMMANDER_LANG") {
        app.insert_resource(GameLanguage::parse(&language).expect("invalid capture language"));
    }

    let mut image = Image::new_uninit(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        // Keep the target metadata/data available to main-world camera
        // maintenance during renderless fast-forwarding, while also uploading
        // it to the render world.
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.insert_resource(CaptureTarget(target.clone()));

    app.finish();
    app.cleanup();
    configure_scenario(app.world_mut(), scenario);
    if scenario.starts_with("settings-audio") {
        // Exercise the real Startup ambient path, normally silent in automated captures.
        app.world_mut().remove_resource::<Autoplay>();
    }
    let mut apps = std::mem::take(app.sub_apps_mut());

    // Publish the render-target asset to the render world before a long
    // main-world-only simulation. Pause gameplay during this startup/render
    // frame so it cannot consume any of the requested simulation time.
    if scenario == "asset-readiness" {
        asset_readiness_capture::install(&mut apps.main);
        println!(
            "[capture] readiness scenario retains cold-start asset loading; no preload warm-up"
        );
    } else {
        warm_render_target(&mut apps);
        println!("[capture] render target warmed");
    }

    if scenario == "picker-settings-layout" {
        picker_settings_capture::install(&mut apps.main);
    } else if scenario.starts_with("onboarding-") {
        onboarding_capture::install(&mut apps.main);
    } else if scenario == "campaign-save-retry" {
        campaign_save_retry_capture::install(&mut apps.main);
    } else if matches!(scenario, "campaign" | "campaign-reload") {
        campaign_capture::install(&mut apps.main);
    }
    if scenario.starts_with("immediate-") {
        immediate_choice_capture::install(&mut apps.main);
    }
    if scenario == "selection-details" {
        selection_detail_capture::install(&mut apps.main);
    }
    if scenario == "gate-revival" {
        gate_revival_capture::install(&mut apps.main);
    }
    if scenario.starts_with("skill-identity-") {
        skill_identity_capture::install(&mut apps.main);
    }
    if ur_evolution_capture::accepts(scenario) {
        ur_evolution_capture::install(&mut apps.main);
    }
    if scenario == "tidal-wave" {
        tidal_wave_capture::install(&mut apps.main);
    }
    if scenario == "cultivation-combat" {
        cultivation_combat_capture::install(&mut apps.main);
    }
    if scenario == "bond-collection" {
        bond_collection_capture::install(&mut apps.main);
    }
    if scenario == "spirit-gallery" {
        spirit_gallery_capture::install(&mut apps.main);
    }
    if scenario == "summon-assault" {
        summon_assault_capture::install(&mut apps.main);
    }
    if scenario == "hero-fusion" {
        fusion_capture::install(&mut apps.main);
    }
    if scenario.starts_with("growth-") {
        growth_capture::install(&mut apps.main);
    }
    if scenario == "bond-opening" {
        bond_opening_capture::install(&mut apps.main);
    }

    if scenario.starts_with("skill-spectacle-") {
        skill_spectacle_capture::install(&mut apps.main);
    }

    if scenario.starts_with("ultimate-") {
        ultimate_capture::install(&mut apps.main);
    }

    if scenario.starts_with("bond-completion") {
        bond_completion_capture::install(&mut apps.main);
    }
    if scenario == "skill-growth" {
        skill_growth_capture::install(&mut apps.main);
    }
    if scenario == "hud-input" {
        hud_input_capture::install(&mut apps.main);
    }
    if scenario == "demon-cave" {
        demon_cave_capture::install(&mut apps.main);
    }
    if scenario.starts_with("raid-pressure-") {
        raid_pressure_capture::install(&mut apps.main);
    }
    if scenario.starts_with("elite-pressure-") {
        elite_pressure_capture::install(&mut apps.main);
    }
    if scenario.starts_with("boss-battle-") {
        boss_battle_capture::install(&mut apps.main);
    }
    if scenario.starts_with("crafting-") {
        crafting_capture::install(&mut apps.main);
    }
    if scenario == "equipment" {
        equipment_capture::install(&mut apps.main);
    }
    if scenario == "attribute-growth" {
        attribute_growth_capture::install(&mut apps.main);
    }
    if scenario == "hero-attributes" {
        hero_attributes_capture::install(&mut apps.main);
    }
    if scenario == "orbit-blade" {
        orbit_blade_capture::install(&mut apps.main);
    }
    if scenario == "enemy-attacks" {
        enemy_attacks_capture::install(&mut apps.main);
    }
    if scenario.starts_with("recruit-") {
        recruitment_capture::install(&mut apps.main);
    }
    if scenario.starts_with("trials-") {
        trial_capture::install(&mut apps.main);
    }
    if scenario == "text-idle" {
        text_idle_capture::install(&mut apps.main);
    } else if scenario == "settings-navigation" {
        settings_navigation_capture::install(&mut apps.main);
    } else if scenario.starts_with("settings-audio") {
        audio_settings_capture::install(
            &mut apps.main,
            scenario == "settings-audio-reload",
            scenario == "settings-audio-failure",
        );
    } else if scenario.starts_with("settings-") {
        settings_capture::install(&mut apps, scenario);
    }

    if scenario.starts_with("attack-") || scenario.starts_with("weapon-growth-") {
        attack_capture::install(&mut apps.main, scenario);
    }

    if scenario == "hud-hint" {
        apps.main
            .add_systems(PreUpdate, show_bond_hint.after(bevy::ui::UiSystems::Focus));
    }

    if scenario.starts_with("build-chain-") {
        apps.main.world_mut().remove_resource::<Autoplay>();
        bond_capture::install(&mut apps.main);
    } else if scenario.starts_with("build-") {
        apps.main.world_mut().remove_resource::<Autoplay>();
        apps.main.world_mut().insert_resource(BuildCapture {
            scenario: scenario.to_owned(),
            frame: 0,
        });
        apps.main.add_systems(
            PreUpdate,
            exercise_build_draft.after(bevy::ui::UiSystems::Focus),
        );
    }

    if scenario.starts_with("draft") {
        apps.main.world_mut().remove_resource::<Autoplay>();
        apps.main.world_mut().insert_resource(DraftCapture {
            scenario: scenario.to_owned(),
            frame: 0,
        });
        apps.main.add_systems(
            PreUpdate,
            exercise_commander_draft.after(bevy::ui::UiSystems::Focus),
        );
    }

    if scenario == "balance-cadence" {
        balance_cadence::install(&mut apps.main);
    }
    if scenario.starts_with("class-") {
        class_attacks_capture::install(&mut apps.main);
    }

    if scenario.starts_with("adventure-") {
        adventure_boss_capture::install(&mut apps.main);
    }

    if mode == "simulate" {
        let simulate_time_receiver = run_simulation(&mut apps, frame_count);
        if let Some(time_receiver) = simulate_time_receiver {
            apps.main.world_mut().insert_resource(time_receiver);
        }
        refresh_render_assets(apps.main.world_mut());
        // Extract the final main-world state and give render-world systems a
        // few frames to rebuild transient view state after renderless updates.
        for _ in 0..SIMULATE_RENDER_SETTLE_FRAMES {
            update_and_wait(&mut apps);
        }
        println!(
            "[capture] final simulation state synchronized to renderer after {SIMULATE_RENDER_SETTLE_FRAMES} settle frames"
        );
        queue_screenshot(apps.main.world_mut(), target.clone(), output.clone());
        wait_for_screenshots(&mut apps);
    } else {
        let timing_path = env::var_os("ROGUE_CAPTURE_TIMINGS").map(PathBuf::from);
        let mut timings =
            String::from("frame,update_and_gpu_wait_ms,entities,enemies,projectiles\n");
        for frame in 0..frame_count {
            let should_save = if mode == "frames" {
                selected_frames
                    .as_ref()
                    .is_none_or(|selected| selected.contains(&frame))
            } else {
                frame + 1 == frame_count
            };
            if should_save {
                let path = if mode == "frames" {
                    output.join(format!("frame{frame:05}.png"))
                } else {
                    output.clone()
                };
                queue_screenshot(apps.main.world_mut(), target.clone(), path);
            }
            let started = std::time::Instant::now();
            update_and_wait(&mut apps);
            let elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;
            if timing_path.is_some() {
                use std::fmt::Write;
                let world = apps.main.world_mut();
                let entities = world.entities().len();
                let enemies = world
                    .query_filtered::<Entity, With<Enemy>>()
                    .iter(world)
                    .count();
                let projectiles = world
                    .query_filtered::<Entity, With<Projectile>>()
                    .iter(world)
                    .count();
                writeln!(
                    timings,
                    "{frame},{elapsed_ms:.6},{entities},{enemies},{projectiles}"
                )
                .expect("format capture timing");
            }
        }
        if let Some(path) = timing_path {
            fs::write(&path, timings).expect("write capture timings");
            println!(
                "[capture] serialized update + GPU wait timings: {} (not windowed FPS)",
                path.display()
            );
        }
        wait_for_screenshots(&mut apps);
    }
    audit_world_material_finish(apps.main.world());
    println!(
        "[capture] wrote {} simulation frames to {}",
        frame_count,
        output.display()
    );
}

fn audit_world_material_finish(world: &World) {
    let server = world.resource::<AssetServer>();
    let materials = world.resource::<Assets<StandardMaterial>>();
    let mut count = 0;
    for (id, material) in materials.iter() {
        let Some(path) = server.get_path(id) else {
            continue;
        };
        let path = path.path().to_string_lossy();
        if !path.starts_with("models/") || !path.ends_with(".glb") || material.unlit {
            continue;
        }
        let (roughness, reflectance) = if material.metallic > 0.5 {
            (0.58, 0.36)
        } else {
            (0.88, 0.24)
        };
        assert!(material.perceptual_roughness >= roughness);
        assert!(material.reflectance <= reflectance + f32::EPSILON);
        assert_eq!(material.clearcoat, 0.0);
        count += 1;
    }
    assert!(count > 0, "capture must load real world model materials");
    println!(
        "[capture] PASS material finish: {count} live imported materials, matte roughness and reduced reflectance"
    );
}

fn refresh_render_assets(world: &mut World) {
    let images = mark_assets_changed::<Image>(world);
    let meshes = mark_assets_changed::<Mesh>(world);
    let materials = mark_assets_changed::<StandardMaterial>(world);
    println!(
        "[capture] refreshed render assets after fast-forward: images={images} meshes={meshes} materials={materials}"
    );
}

fn mark_assets_changed<A: Asset>(world: &mut World) -> usize {
    let mut assets = world.resource_mut::<Assets<A>>();
    let count = assets.len();
    for _ in assets.iter_mut() {}
    count
}

fn warm_render_target(apps: &mut SubApps) {
    let (was_paused, elapsed_before) = {
        let session = apps.main.world().resource::<GameSession>();
        (session.paused, session.elapsed)
    };
    apps.main.world_mut().resource_mut::<GameSession>().paused = true;
    update_and_wait(apps);
    // Forced capture scenarios bypass the hero picker that normally gives assets
    // time to load. Wait for actual scene dependencies before recording frame 0.
    let mut settled = 0;
    for _ in 0..600 {
        let world = apps.main.world_mut();
        let scene_ids: Vec<_> = world
            .query::<&WorldAssetRoot>()
            .iter(world)
            .map(|root| root.0.id())
            .collect();
        let server = world.resource::<AssetServer>();
        let cast = world.resource::<GameAssets>();
        let ready = cast
            .hero_scenes
            .iter()
            .chain(&cast.enemy_scenes)
            .chain(&cast.boss_scenes)
            .chain(&cast.summon_scenes)
            .all(|scene| server.is_loaded_with_dependencies(scene.id()))
            && scene_ids
                .iter()
                .all(|id| server.is_loaded_with_dependencies(*id));
        settled = if ready { settled + 1 } else { 0 };
        update_and_wait(apps);
        if settled == SIMULATE_BOOTSTRAP_RENDER_FRAMES {
            break;
        }
    }
    assert_eq!(
        settled, SIMULATE_BOOTSTRAP_RENDER_FRAMES,
        "capture scene assets failed to load"
    );
    println!("[capture] authored scene dependencies loaded and renderer settled");
    let mut session = apps.main.world_mut().resource_mut::<GameSession>();
    let elapsed_after = session.elapsed;
    session.paused = was_paused;
    assert_eq!(
        elapsed_before.to_bits(),
        elapsed_after.to_bits(),
        "render-target warm-up advanced gameplay time"
    );
    println!(
        "[capture] warm-up elapsed unchanged at {elapsed_after:.3}s (paused restored to {was_paused})"
    );
}

fn queue_screenshot(world: &mut World, target: Handle<Image>, path: PathBuf) {
    if path.exists() {
        fs::remove_file(&path)
            .unwrap_or_else(|error| panic!("remove stale capture {}: {error}", path.display()));
    }
    world.resource_mut::<CaptureSaveStatus>().expected += 1;

    let mut save = save_to_disk(path.clone());
    world.spawn(Screenshot::image(target)).observe(
        move |captured: On<ScreenshotCaptured>, mut status: ResMut<CaptureSaveStatus>| {
            save(captured);
            match fs::metadata(&path) {
                Ok(metadata) if metadata.len() > 0 => {
                    status.saved += 1;
                    println!(
                        "[capture] saved {}/{} {} ({} bytes)",
                        status.saved,
                        status.expected,
                        path.display(),
                        metadata.len()
                    );
                }
                Ok(_) => {
                    status.failed += 1;
                    eprintln!("[capture] save failed: {} is empty", path.display());
                }
                Err(error) => {
                    status.failed += 1;
                    eprintln!(
                        "[capture] save failed: {} was not created: {error}",
                        path.display()
                    );
                }
            }
        },
    );
}

fn wait_for_screenshots(apps: &mut SubApps) {
    let expected = apps.main.world().resource::<CaptureSaveStatus>().expected;
    if expected == 0 {
        println!("[capture] no screenshots requested");
        return;
    }

    for wait_frame in 1..=CAPTURE_WAIT_FRAMES {
        update_and_wait(apps);
        let status = apps.main.world().resource::<CaptureSaveStatus>();
        if status.saved + status.failed < status.expected {
            continue;
        }
        if status.failed > 0 {
            panic!(
                "[capture] save completed with failures: saved={} failed={} expected={}",
                status.saved, status.failed, status.expected
            );
        }
        println!(
            "[capture] all {} screenshot(s) saved after {wait_frame} wait frame(s)",
            status.saved
        );
        return;
    }

    let status = apps.main.world().resource::<CaptureSaveStatus>();
    eprintln!(
        "[capture] save timeout after {CAPTURE_WAIT_FRAMES} frames: saved={} failed={} expected={}",
        status.saved, status.failed, status.expected
    );
    panic!("capture save timeout");
}

fn configure_scenario(world: &mut World, scenario: &str) {
    if scenario == "picker-settings-layout" {
        picker_settings_capture::configure(world);
        return;
    }
    if scenario.starts_with("onboarding-") {
        onboarding_capture::configure(world, scenario);
        return;
    }
    if scenario == "asset-readiness" {
        asset_readiness_capture::configure(world);
        return;
    }
    if scenario == "campaign-save-retry" {
        campaign_save_retry_capture::configure(world);
        return;
    }
    if matches!(scenario, "campaign" | "campaign-reload") {
        campaign_capture::configure(world, scenario == "campaign-reload");
        return;
    }
    if !scenario.starts_with("campaign") {
        world.resource_mut::<CampaignMenu>().0 = false;
        world.resource_mut::<GameSession>().paused = false;
    }
    if scenario == "balance-cadence" {
        balance_cadence::configure(world);
        return;
    }
    if scenario.starts_with("class-") {
        class_attacks_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("adventure-") {
        adventure_boss_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("immediate-") {
        immediate_choice_capture::configure(world, scenario);
        return;
    }
    if scenario == "selection-details" {
        selection_detail_capture::configure(world);
        return;
    }
    if scenario == "gate-revival" {
        gate_revival_capture::configure(world);
        return;
    }
    if scenario.starts_with("skill-identity-") {
        skill_identity_capture::configure(world, scenario);
        return;
    }
    if ur_evolution_capture::accepts(scenario) {
        ur_evolution_capture::configure(world, scenario);
        return;
    }
    if scenario == "tidal-wave" {
        tidal_wave_capture::configure(world);
        return;
    }
    if scenario == "spirit-gallery" {
        spirit_gallery_capture::configure(world);
        return;
    }
    if scenario == "summon-assault" {
        summon_assault_capture::configure(world);
        return;
    }
    if scenario == "hero-fusion" {
        fusion_capture::configure(world);
        return;
    }
    if scenario == "cultivation-combat" {
        cultivation_combat_capture::configure(world);
        return;
    }
    if scenario == "bond-collection" {
        bond_collection_capture::configure(world);
        return;
    }
    if scenario.starts_with("growth-") {
        growth_capture::configure(world, scenario);
        return;
    }
    if scenario == "enemy-attacks" {
        enemy_attacks_capture::configure(world);
        return;
    }
    if scenario == "hud-input" {
        hero_attributes_capture::configure(world);
        return;
    }
    if scenario == "demon-cave" {
        demon_cave_capture::configure(world);
        return;
    }
    if scenario.starts_with("raid-pressure-") {
        raid_pressure_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("elite-pressure-") {
        elite_pressure_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("boss-battle-") {
        boss_battle_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("crafting-") {
        crafting_capture::configure(world, scenario);
        return;
    }
    if scenario == "equipment" {
        equipment_capture::configure(world);
        return;
    }
    if scenario == "attribute-growth" {
        attribute_growth_capture::configure(world);
        return;
    }
    if scenario == "hero-attributes" {
        hero_attributes_capture::configure(world);
        return;
    }
    if scenario == "orbit-blade" {
        orbit_blade_capture::configure(world);
        return;
    }
    if scenario == "bond-opening" {
        bond_opening_capture::configure(world);
        return;
    }
    if scenario.starts_with("skill-spectacle-") {
        skill_spectacle_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("ultimate-") {
        ultimate_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("bond-completion") {
        bond_completion_capture::configure(world, scenario);
        return;
    }
    if scenario == "skill-growth" {
        skill_growth_capture::configure(world);
        return;
    }
    if scenario.starts_with("recruit-") {
        recruitment_capture::configure(world, scenario);
        return;
    }
    if scenario.starts_with("trials-") {
        trial_capture::configure(world, scenario);
        return;
    }

    println!("[capture] scenario={scenario}");
    println!("[capture] seed={:#018X}", world.resource::<RunSeed>().0);
    if scenario.starts_with("weapon-growth-") {
        attack_capture::configure_growth(world, scenario);
        return;
    }
    if let Some(weapon) = scenario.strip_prefix("attack-") {
        let weapon = weapon.strip_suffix("-fragile").unwrap_or(weapon);
        use rogue_commander::game::HeroKind;
        let kind = match weapon {
            "stone" | "sword" | "warpath" => HeroKind::StoneWarden,
            "iron" | "axe" => HeroKind::IronBerserker,
            "rift" | "daggers" => HeroKind::RiftDancer,
            "ember" | "bolt" | "archery" => HeroKind::EmberRanger,
            "storm" | "arcanum" => HeroKind::StormSavant,
            "spirit" => HeroKind::SpiritKeeper,
            "star" => HeroKind::StarWagerer,
            "frost" => HeroKind::FrostOracle,
            "solar" => HeroKind::SolarVanguard,
            "tide" => HeroKind::TideWalker,
            "void" => HeroKind::VoidReaper,
            "grove" => HeroKind::GroveArtificer,
            _ => panic!("unknown attack capture: {weapon}"),
        };
        let mut selected = world.resource_mut::<SelectedHero>();
        selected.kind = kind;
        selected.locked = true;
        let mut family = match weapon {
            "warpath" => Some(BondId::Warpath),
            "archery" => Some(BondId::Archery),
            "arcanum" => Some(BondId::Arcanum),
            _ => None,
        };
        while let Some(current) = family {
            for card in current.definition().members {
                world.resource_mut::<PlayerBuild>().owned_bond_cards[card.index()] = true;
            }
            family = current.definition().prerequisite;
        }
        world.resource_mut::<CaptureControls>().suppress_drafts = true;
        world.resource_mut::<GameSession>().auto_bond_draws = false;
        world.resource_mut::<SpawnDirector>().enabled = false;
        return;
    }
    if (scenario.starts_with("settings-") && !scenario.starts_with("settings-recruit"))
        || scenario.starts_with("build-")
        || scenario.starts_with("hud")
        || matches!(scenario, "balance" | "showcase" | "final" | "summons")
    {
        world.resource_mut::<SelectedHero>().locked = true;
        println!("[capture] selected hero locked for configured scenario");
    }
    if (scenario.starts_with("settings-") && !scenario.starts_with("settings-recruit"))
        || scenario.starts_with("build-")
        || scenario.starts_with("hud")
    {
        let mut session = world.resource_mut::<GameSession>();
        session.elapsed = 20.0;
        session.wood = 60;
        session.gold = 120;
        session.auto_bond_draws = false;
        if scenario.starts_with("hud") {
            world.resource_mut::<CaptureControls>().suppress_drafts = true;
        }
        if scenario.starts_with("build-chain-") {
            bond_capture::configure(world, scenario);
        }
        if scenario == "hud-build" {
            let mut build = world.resource_mut::<PlayerBuild>();
            for family in [BondId::Journey, BondId::Agility] {
                for card in family.definition().members {
                    build.owned_bond_cards[card.index()] = true;
                }
            }
            build.owned_bond_cards[BondId::Ranger.definition().members[0].index()] = true;
        }
        return;
    }
    if scenario == "balance" {
        let mut build = world.resource_mut::<PlayerBuild>();
        build.attack_damage = 48.0;
        build.attacks_per_second = 1.95;
        build.projectile_speed = 19.2;
        build.attack_range = 9.2;
        build.move_speed = 6.1;
        build.skills[0] = 1;
        world.resource_mut::<TrialState>().auto = true;
        world.resource_mut::<GoldChallengeState>().auto = true;
        println!(
            "[capture] balance bot configured with the default hero starter skill and both auto challenges"
        );
        return;
    }
    if scenario == "summons" {
        world.resource_mut::<CaptureControls>().suppress_drafts = true;
        {
            let mut session = world.resource_mut::<GameSession>();
            session.elapsed = 480.0;
            session.level = 15;
            session.xp = 0;
            session.xp_next = u32::MAX;
            session.skill_points = 0;
            session.gold = 1_200;
            session.wood = 120;
            session.core_max_hp = 2_000.0;
            session.core_hp = 2_000.0;
        }
        {
            let mut build = world.resource_mut::<PlayerBuild>();
            build.attack_damage = 180.0;
            build.attacks_per_second = 2.4;
            build.projectile_speed = 22.0;
            build.attack_range = 16.0;
            build.skills[4] = 5;
            build.skills[5] = 5;
            build.skills[16] = 3;
            build.skills[17] = 3;
            build.skills[26] = 2;
            build.skills[27] = 2;
        }
        world.resource_mut::<SpawnDirector>().boss_spawned[0] = true;
        println!("[capture] six-summon visual verification scenario configured");
        return;
    }
    if !matches!(scenario, "showcase" | "final") {
        return;
    }
    world.resource_mut::<CaptureControls>().suppress_drafts = scenario == "final";
    {
        let mut session = world.resource_mut::<GameSession>();
        session.elapsed = if scenario == "final" { 1_500.0 } else { 299.0 };
        session.level = if scenario == "final" { 30 } else { 9 };
        session.xp = 0;
        session.xp_next = if scenario == "final" {
            u32::MAX
        } else {
            60 + 18 * (session.level - 1)
        };
        session.skill_points = 0;
        session.auto_bond_draws = false;
        session.gold = 2_400;
        session.wood = 180;
        session.kills = 1_600;
        session.core_max_hp = 2_000.0;
        session.core_hp = 2_000.0;
        if scenario == "final" {
            session.bosses_defeated = 4;
        }
    }
    {
        let mut build = world.resource_mut::<PlayerBuild>();
        // Let the final capture show both phases instead of spending half of
        // a 16-second delivery clip on a static result screen.
        build.attack_damage = if scenario == "final" { 2_500.0 } else { 310.0 };
        build.attacks_per_second = if scenario == "final" { 3.5 } else { 4.2 };
        build.projectile_speed = 24.0;
        build.attack_range = if scenario == "final" { 42.0 } else { 17.0 };
        build.multishot = if scenario == "final" { 3 } else { 4 };
        build.pierce = if scenario == "final" { 0 } else { 4 };
        for index in [0, 3, 4, 5, 6, 12, 16, 17, 18, 24, 26, 27] {
            build.skills[index] = if scenario == "final" { 2 } else { 3 };
        }
        // Harness state control: completed prerequisites, not a simulated draw sequence.
        for family in [BondId::Journey, BondId::Agility, BondId::Ranger] {
            for card in family.definition().members {
                build.owned_bond_cards[card.index()] = true;
            }
        }
    }
    let mut director = world.resource_mut::<SpawnDirector>();
    director.next_elite_at = if scenario == "final" { 1_520.0 } else { 360.0 };
    if scenario == "final" {
        director.boss_spawned[..4].fill(true);
    }
    println!("[capture] scenario state configured");
}

fn run_simulation(apps: &mut SubApps, frame_count: u32) -> Option<TimeReceiver> {
    let mut simulated = 0;
    let mut time_receiver = None;
    for frame in 0..frame_count {
        if frame == SIMULATE_BOOTSTRAP_RENDER_FRAMES {
            // ManualDuration does not need render timestamps. Detach the
            // receiver while rendering is skipped so time_system does not
            // warn on every main-only frame; restore it before final render.
            time_receiver = apps.main.world_mut().remove_resource::<TimeReceiver>();
        }
        if frame < SIMULATE_BOOTSTRAP_RENDER_FRAMES {
            // Let startup-created assets, views, and UI reach the render world.
            // This cost is fixed; long simulations remain main-world-only.
            update_and_wait(apps);
        } else {
            // Match SubApps::update's main-world phase without prematurely
            // clearing Added/Changed trackers needed by final extraction.
            apps.main.run_default_schedule();
        }
        simulated = frame + 1;
        let should_checkpoint =
            simulated == 1 || simulated.is_multiple_of(1_800) || simulated == frame_count;
        if should_checkpoint {
            print_simulation_checkpoint(apps.main.world_mut(), simulated);
        }
        if apps.main.world().resource::<GameSession>().finished {
            if !should_checkpoint {
                print_simulation_checkpoint(apps.main.world_mut(), simulated);
            }
            break;
        }
    }
    println!("[capture] simulation stopped after {simulated} frames");
    time_receiver
}

fn print_simulation_checkpoint(world: &mut World, frame: u32) {
    let (
        elapsed,
        wave,
        core_hp,
        core_max_hp,
        level,
        xp,
        xp_next,
        gold,
        kill_credit,
        total_kills,
        wood,
        bosses,
        save,
        save_remaining,
        finished,
        victory,
    ) = {
        let session = world.resource::<GameSession>();
        (
            session.elapsed,
            session.wave,
            session.core_hp,
            session.core_max_hp,
            session.level,
            session.xp,
            session.xp_next,
            session.gold,
            session.kills,
            session.total_kills,
            session.wood,
            session.bosses_defeated,
            session.save_active,
            session.save_remaining,
            session.finished,
            session.victory,
        )
    };
    let seed = world.resource::<RunSeed>().0;
    let rng_state = world.resource::<RunRng>().state();
    let (skill_slots, skill_ranks, bond_families, bond_cards, completed_bonds) = {
        let build = world.resource::<PlayerBuild>();
        (
            build.skills.iter().filter(|rank| **rank > 0).count(),
            build
                .skills
                .iter()
                .map(|rank| usize::from(*rank))
                .sum::<usize>(),
            build.active_bond_count(),
            build
                .owned_bond_cards
                .iter()
                .filter(|owned| **owned)
                .count(),
            BondId::ALL
                .iter()
                .filter(|family| build.bond_complete(**family))
                .count(),
        )
    };
    let (tower_level, tower_active, gold_level, gold_active, gold_targets) = {
        let tower = world.resource::<TrialState>();
        let gold = world.resource::<GoldChallengeState>();
        (
            tower.level,
            tower.active,
            gold.level,
            gold.active,
            gold.remaining_targets,
        )
    };
    let (archive_active, archive_outcome) = {
        let archive = world.resource::<ArchiveState>();
        (archive.active, archive.outcome)
    };
    let enemies = world
        .query_filtered::<Entity, With<Enemy>>()
        .iter(world)
        .count();
    let projectiles = world
        .query_filtered::<Entity, With<Projectile>>()
        .iter(world)
        .count();
    let pool_snapshot = world.resource::<CombatPools>().snapshot();
    let cameras: Vec<(bool, Option<UVec2>)> = world
        .query::<&Camera>()
        .iter(world)
        .map(|camera| (camera.is_active, camera.physical_target_size()))
        .collect();
    println!(
        "[capture] checkpoint seed={seed:#018X} rng={rng_state:#018X} frame={frame} elapsed={elapsed:.1}s wave={wave} core={core_hp:.0}/{core_max_hp:.0} level={level} xp={xp}/{xp_next} gold={gold} kill_credit={kill_credit} total_kills={total_kills} wood={wood} bosses={bosses} skills={skill_slots}/{skill_ranks} bonds={bond_families}/{bond_cards}/{completed_bonds} tower={tower_level}/{tower_active} gold_trial={gold_level}/{gold_active}/{gold_targets} enemies={enemies} projectiles={projectiles} pool_enemy={}/{}/{}/{}/{} pool_projectile_total={}/{}/{}/{}/{} pool_projectile_visible={}/{}/{}/{}/{} projectile_visual_fallbacks={} pool_fx={}/{}/{}/{}/{} pool_summon={}/{}/{}/{}/{} pool_damage_text={}/{}/{}/{}/{}/{} cameras={cameras:?} save={save}/{save_remaining:.1} archive={archive_active}/{archive_outcome:?} finished={finished} victory={victory}",
        pool_snapshot.enemies.created(),
        pool_snapshot.enemies.reused(),
        pool_snapshot.enemies.active(),
        pool_snapshot.enemies.peak(),
        pool_snapshot.enemies.rejected(),
        pool_snapshot.projectiles.created(),
        pool_snapshot.projectiles.reused(),
        pool_snapshot.projectiles.active(),
        pool_snapshot.projectiles.peak(),
        pool_snapshot.projectiles.rejected(),
        pool_snapshot.projectile_visible.created(),
        pool_snapshot.projectile_visible.reused(),
        pool_snapshot.projectile_visible.active(),
        pool_snapshot.projectile_visible.peak(),
        pool_snapshot.projectile_visible.rejected(),
        pool_snapshot.projectile_visual_fallbacks,
        pool_snapshot.impact_fx.created(),
        pool_snapshot.impact_fx.reused(),
        pool_snapshot.impact_fx.active(),
        pool_snapshot.impact_fx.peak(),
        pool_snapshot.impact_fx.rejected(),
        pool_snapshot.summons.created(),
        pool_snapshot.summons.reused(),
        pool_snapshot.summons.active(),
        pool_snapshot.summons.peak(),
        pool_snapshot.summons.rejected(),
        pool_snapshot.damage_numbers.created(),
        pool_snapshot.damage_numbers.reused(),
        pool_snapshot.damage_numbers.active(),
        pool_snapshot.damage_numbers.peak(),
        pool_snapshot.damage_numbers.coalesced(),
        pool_snapshot.damage_numbers.overwritten(),
    );
}

fn suppress_final_drafts(
    mut commands: Commands,
    controls: Res<CaptureControls>,
    mut session: ResMut<GameSession>,
    mut draft: ResMut<DraftState>,
    roots: Query<Entity, With<DraftRoot>>,
) {
    if !controls.suppress_drafts {
        return;
    }
    let had_draft = !roots.is_empty() || draft.root.is_some();
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    draft.root = None;
    draft.choices.clear();
    draft.kind = None;
    draft.opened_for = 0.0;
    session.skill_points = 0;
    if had_draft {
        session.paused = false;
    }
}

fn update_and_wait(apps: &mut SubApps) {
    apps.update();
    apps.main
        .world()
        .resource::<RenderDevice>()
        .wgpu_device()
        .poll(PollType::Wait {
            submission_index: None,
            timeout: None,
        })
        .expect("poll render device");
}
