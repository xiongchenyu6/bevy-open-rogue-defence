//! Responsive framing keeps the battlefield angle fixed and pans only at HUD edges.
use bevy::{
    camera::{RenderTarget, ScalingMode},
    prelude::*,
    window::{PrimaryWindow, WindowRef},
};

use super::super::{ControlScheme, GameSession, Hero, mobile_layout::hud_metrics};

const EYE: Vec3 = Vec3::new(-20.0, 25.2, 10.0);
const FOCUS: Vec3 = Vec3::new(4.0, 1.2, 0.0);
const BODY_CENTER_HEIGHT: f32 = 1.2;

/// Production camera ownership. Explicit capture camera proposals remove this marker.
/// The previous unshaken base lets screen shake retain its own additive offset.
#[derive(Component)]
pub struct BattleCamera {
    previous_base: Vec3,
    previous_size: Vec2,
}

impl Default for BattleCamera {
    fn default() -> Self {
        Self {
            previous_base: EYE,
            previous_size: Vec2::ZERO,
        }
    }
}

pub(super) fn initial_transform() -> Transform {
    Transform::from_translation(EYE).looking_at(FOCUS, Vec3::Y)
}

fn vertical_span(size: Vec2) -> f32 {
    28.0_f32.max(22.0 * size.y / size.x)
}

fn ground_up() -> Vec3 {
    let up = *initial_transform().up();
    Vec3::new(up.x, 0.0, up.z).normalize()
}

/// Camera translation required to move the rendered world by a logical-pixel delta.
fn screen_shift(delta: Vec2, size: Vec2) -> Vec3 {
    let pose = initial_transform();
    let pixels_per_meter = size.y / vertical_span(size);
    let up = ground_up();
    -*pose.right() * (delta.x / pixels_per_meter)
        + up * (delta.y / (pixels_per_meter * pose.up().dot(up)))
}

fn responsive_base(size: Vec2) -> Vec3 {
    let compact_landscape = size.x >= size.y && hud_metrics(size.x, size.y).compact;
    EYE + if compact_landscape {
        screen_shift(Vec2::new(0.0, 24.0), size)
    } else {
        Vec3::ZERO
    }
}

fn fit_axis(min: f32, max: f32, extent: f32) -> (f32, f32) {
    let minimum_span = 64.0_f32.min(extent);
    let min = min.clamp(0.0, extent - minimum_span);
    (min, max.clamp(min + minimum_span, extent))
}

fn gameplay_rect(size: Vec2, touch: bool) -> Rect {
    let layout = hud_metrics(size.x, size.y);
    let (left, top, right, bottom) = if size.y > size.x {
        (28.0, 270.0, size.x - 28.0, size.y - 100.0)
    } else if layout.compact {
        (
            layout.safe_x + 220.0 + 20.0,
            125.0,
            size.x - 60.0,
            if touch {
                // Compact touch uses a stacked XP/skill panel and large dodge
                // control. Keep the body above the entire fixed footer band,
                // with room for its feet, rather than just above the skill button.
                size.y - layout.safe_y - layout.joystick_size - 24.0
            } else {
                size.y - 65.0
            },
        )
    } else {
        (280.0, 120.0, size.x - 30.0, size.y - 90.0)
    };
    let (left, right) = fit_axis(left, right, size.x);
    let (top, bottom) = fit_axis(top, bottom, size.y);
    Rect::from_corners(Vec2::new(left, top), Vec2::new(right, bottom))
}

fn projected(point: Vec3, base: Vec3, size: Vec2) -> Vec2 {
    let pose = initial_transform();
    let relative = point - base;
    let scale = size.y / vertical_span(size);
    size * 0.5 + Vec2::new(relative.dot(*pose.right()), -relative.dot(*pose.up())) * scale
}

fn hero_base(point: Vec3, size: Vec2, touch: bool) -> Vec3 {
    let base = responsive_base(size);
    let pixel = projected(point, base, size);
    let safe = gameplay_rect(size, touch);
    base + screen_shift(pixel.clamp(safe.min, safe.max) - pixel, size)
}

pub(super) fn update_battle_camera(
    windows: Query<&Window, With<PrimaryWindow>>,
    session: Res<GameSession>,
    scheme: Res<ControlScheme>,
    heroes: Query<&Transform, (With<Hero>, Without<BattleCamera>)>,
    mut cameras: Query<(&mut Transform, &mut Projection, &mut BattleCamera), Without<Hero>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let size = Vec2::new(window.width().max(1.0), window.height().max(1.0));
    for (mut transform, mut projection, mut camera) in &mut cameras {
        let span = vertical_span(size);
        let needs_projection_update = match projection.as_ref() {
            Projection::Orthographic(orthographic) => match orthographic.scaling_mode {
                ScalingMode::FixedVertical { viewport_height } => viewport_height != span,
                _ => true,
            },
            _ => continue,
        };
        // A mutable dereference alone marks Projection changed. Do not dirty
        // the 3D camera every frame while its same-target 2D overlay waits for
        // the actual resize event to refresh its render-target size.
        if needs_projection_update && let Projection::Orthographic(orthographic) = &mut *projection
        {
            orthographic.scaling_mode = ScalingMode::FixedVertical {
                viewport_height: span,
            };
        }
        let next_base = if (session.paused || session.finished) && camera.previous_size == size {
            camera.previous_base
        } else if session.paused || session.finished {
            // A resize changes framing, but a paused hero cannot move the camera.
            let previous_responsive = if camera.previous_size == Vec2::ZERO {
                EYE
            } else {
                responsive_base(camera.previous_size)
            };
            responsive_base(size) + camera.previous_base - previous_responsive
        } else if let Ok(hero) = heroes.single() {
            hero_base(
                hero.translation + Vec3::Y * BODY_CENTER_HEIGHT,
                size,
                scheme.touch(),
            )
        } else {
            responsive_base(size)
        };
        // Never overwrite the current transform: it can still contain last
        // frame's shake, which apply_screen_shake removes after this system.
        let delta = next_base - camera.previous_base;
        if delta != Vec3::ZERO {
            transform.translation += delta;
        }
        camera.previous_base = next_base;
        camera.previous_size = size;
    }
}

/// Programmatic display-mode restoration can change Window.resolution before
/// Winit emits WindowResized. Refresh every camera sharing that primary window
/// together, including the damage-number overlay, before Bevy computes targets.
pub(super) fn sync_primary_window_camera_sizes(
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut cameras: Query<(&Camera, &RenderTarget, &mut Projection)>,
) {
    let Ok((primary, window)) = windows.single() else {
        return;
    };
    let size = window.physical_size();
    for (camera, target, mut projection) in &mut cameras {
        let primary_target = match target {
            RenderTarget::Window(WindowRef::Primary) => true,
            RenderTarget::Window(WindowRef::Entity(entity)) => *entity == primary,
            _ => false,
        };
        if primary_target && camera.physical_target_size() != Some(size) {
            projection.set_changed();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::CameraProjection;

    fn bevy_projected(point: Vec3, base: Vec3, size: Vec2) -> Vec2 {
        let mut projection = OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: vertical_span(size),
            },
            ..OrthographicProjection::default_3d()
        };
        projection.update(size.x, size.y);
        let mut pose = initial_transform();
        pose.translation = base;
        let ndc =
            (projection.get_clip_from_view() * pose.to_matrix().inverse()).project_point3(point);
        Vec2::new((ndc.x + 1.0) * size.x * 0.5, (1.0 - ndc.y) * size.y * 0.5)
    }

    fn fixture(size: (u32, u32), point: Vec3) -> (App, Entity, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<GameSession>()
            .init_resource::<ControlScheme>()
            .add_systems(Update, update_battle_camera);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: size.into(),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        let hero = app
            .world_mut()
            .spawn((Hero, Transform::from_translation(point)))
            .id();
        let camera = app
            .world_mut()
            .spawn((
                initial_transform(),
                Projection::from(OrthographicProjection::default_3d()),
                BattleCamera::default(),
            ))
            .id();
        (app, window, hero, camera)
    }

    #[test]
    fn live_hero_corners_fit_gameplay_rect_without_changing_angle() {
        for size in [
            Vec2::new(1280.0, 720.0),
            Vec2::new(667.0, 375.0),
            Vec2::new(390.0, 844.0),
        ] {
            let safe = gameplay_rect(size, false);
            for x in [-10.0, 10.0] {
                for z in [-12.0, 12.0] {
                    let point = Vec3::new(x, BODY_CENTER_HEIGHT, z);
                    let pixel = bevy_projected(point, hero_base(point, size, false), size);
                    assert!(pixel.x >= safe.min.x - 0.001 && pixel.x <= safe.max.x + 0.001);
                    assert!(pixel.y >= safe.min.y - 0.001 && pixel.y <= safe.max.y + 0.001);
                    assert!((hero_base(point, size, false).y - EYE.y).abs() < 0.001);
                }
            }
        }
        assert!((vertical_span(Vec2::new(390.0, 844.0)) - 22.0 * 844.0 / 390.0).abs() < 0.001);
    }

    #[test]
    fn compact_base_moves_world_down_twenty_four_pixels() {
        let size = Vec2::new(667.0, 375.0);
        let point = Vec3::new(14.0, 1.2, 0.0);
        let delta =
            bevy_projected(point, responsive_base(size), size) - bevy_projected(point, EYE, size);
        assert!((delta - Vec2::new(0.0, 24.0)).length() < 0.001);
        assert_eq!(responsive_base(Vec2::new(1280.0, 720.0)), EYE);
    }

    #[test]
    fn resize_reframes_pause_holds_pan_and_repeated_updates_do_not_drift() {
        let (mut app, window, hero, camera) = fixture((1280, 720), Vec3::new(-10.0, 0.0, 12.0));
        app.update();
        let before = app.world().get::<Transform>(camera).unwrap().translation;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation = Vec3::new(10.0, 0.0, -12.0);
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            before
        );
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(390.0, 844.0);
        app.update();
        let resized = app.world().get::<Transform>(camera).unwrap().translation;
        let Projection::Orthographic(projection) = app.world().get::<Projection>(camera).unwrap()
        else {
            panic!("battle camera must remain orthographic");
        };
        let ScalingMode::FixedVertical { viewport_height } = projection.scaling_mode else {
            panic!("battle camera must preserve fixed vertical framing");
        };
        assert!((viewport_height - 22.0 * 844.0 / 390.0).abs() < 0.001);
        for _ in 0..4 {
            app.update();
            assert_eq!(
                app.world().get::<Transform>(camera).unwrap().translation,
                resized
            );
        }
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        let transform = app.world().get::<Transform>(camera).unwrap();
        assert_eq!(transform.rotation, initial_transform().rotation);
        let point =
            app.world().get::<Transform>(hero).unwrap().translation + Vec3::Y * BODY_CENTER_HEIGHT;
        let pixel = projected(point, transform.translation, Vec2::new(390.0, 844.0));
        let safe = gameplay_rect(Vec2::new(390.0, 844.0), false);
        assert!(pixel.distance(pixel.clamp(safe.min, safe.max)) < 0.001);
    }

    #[test]
    fn base_delta_keeps_prior_shake_and_removed_marker_keeps_preview_fixed() {
        let (mut app, _, _, camera) = fixture((667, 375), Vec3::new(-8.2, 0.0, 0.0));
        let shake = Vec3::new(0.12, -0.04, 0.03);
        app.world_mut()
            .get_mut::<Transform>(camera)
            .unwrap()
            .translation += shake;
        app.update();
        let camera_state = app.world().get::<BattleCamera>(camera).unwrap();
        let base = camera_state.previous_base;
        let translation = app.world().get::<Transform>(camera).unwrap().translation;
        assert!((translation - base - shake).length() < 0.001);
        // Screen shake's subsequent removal of its old offset recovers the new base.
        assert!((translation - shake - base).length() < 0.001);
        app.world_mut().entity_mut(camera).remove::<BattleCamera>();
        let fixed =
            Transform::from_xyz(10.0, 24.0, 21.0).looking_at(Vec3::new(2.0, 0.0, 0.0), Vec3::Y);
        *app.world_mut().get_mut::<Transform>(camera).unwrap() = fixed;
        app.update();
        assert_eq!(*app.world().get::<Transform>(camera).unwrap(), fixed);
    }

    #[test]
    fn compact_touch_corner_keeps_body_above_actual_fixed_footer() {
        let size = Vec2::new(667.0, 375.0);
        // R5 native frame238: fixed control bounds; unlike world camp labels,
        // these rectangles stay put when the camera pans.
        let controls = [
            Rect::from_corners(Vec2::new(46.69, 243.0), Vec2::new(166.69, 363.0)),
            Rect::from_corners(Vec2::new(265.0, 276.0), Vec2::new(455.0, 310.0)),
            Rect::from_corners(Vec2::new(296.0, 315.0), Vec2::new(408.0, 363.0)),
            Rect::from_corners(Vec2::new(536.0, 261.0), Vec2::new(620.0, 345.0)),
        ];
        for x in [-10.0, 10.0] {
            for z in [-12.0, 12.0] {
                let point = Vec3::new(x, BODY_CENTER_HEIGHT, z);
                let pixel = bevy_projected(point, hero_base(point, size, true), size);
                let body_margin = Vec2::splat(24.0);
                let body = Rect::from_corners(pixel - body_margin, pixel + body_margin);
                for control in controls {
                    assert!(body.intersect(control).is_empty());
                }
            }
        }
        let point = Vec3::new(-10.0, BODY_CENTER_HEIGHT, 12.0);
        let old_pixel = bevy_projected(point, hero_base(point, size, false), size);
        assert!((old_pixel.y - 310.0).abs() < 0.001);
        let old_body =
            Rect::from_corners(old_pixel - Vec2::splat(24.0), old_pixel + Vec2::splat(24.0));
        assert!(!old_body.intersect(controls[1]).is_empty());
        let new_pixel = bevy_projected(point, hero_base(point, size, true), size);
        assert!((new_pixel.y - 219.0).abs() < 0.001);
    }

    #[test]
    fn touch_footer_pan_is_compact_landscape_only_and_pause_stays_fixed() {
        for size in [Vec2::new(1280.0, 720.0), Vec2::new(390.0, 844.0)] {
            assert_eq!(gameplay_rect(size, true), gameplay_rect(size, false));
        }
        let (mut app, _, _, camera) = fixture((667, 375), Vec3::new(-10.0, 0.0, 12.0));
        app.update();
        let keyboard = app.world().get::<Transform>(camera).unwrap().translation;
        app.world_mut().resource_mut::<GameSession>().paused = true;
        *app.world_mut().resource_mut::<ControlScheme>() = ControlScheme::Touch;
        app.update();
        assert_eq!(
            app.world().get::<Transform>(camera).unwrap().translation,
            keyboard
        );
        app.world_mut().resource_mut::<GameSession>().paused = false;
        app.update();
        let touch = app.world().get::<Transform>(camera).unwrap().translation;
        assert!(touch.distance(keyboard) > 1.0);
        let point = Vec3::new(-10.0, BODY_CENTER_HEIGHT, 12.0);
        assert!((bevy_projected(point, touch, Vec2::new(667.0, 375.0)).y - 219.0).abs() < 0.001);
    }

    #[derive(Resource, Default)]
    struct CameraChanges(Vec<(bool, bool)>);

    fn record_camera_changes(
        cameras: Query<(Ref<Projection>, Ref<Transform>), With<BattleCamera>>,
        mut changes: ResMut<CameraChanges>,
    ) {
        let (projection, transform) = cameras.single().unwrap();
        changes
            .0
            .push((projection.is_changed(), transform.is_changed()));
    }

    #[test]
    fn unchanged_camera_does_not_dirty_projection_or_transform() {
        let (mut app, window, hero, _) = fixture((1280, 720), Vec3::new(-8.2, 0.0, 0.0));
        app.init_resource::<CameraChanges>()
            .add_systems(Last, record_camera_changes);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<CameraChanges>().0.last(),
            Some(&(false, false))
        );
        app.world_mut()
            .get_mut::<Transform>(hero)
            .unwrap()
            .translation = Vec3::new(-10.0, 0.0, 12.0);
        app.update();
        assert_eq!(
            app.world().resource::<CameraChanges>().0.last(),
            Some(&(false, true))
        );
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(390.0, 844.0);
        app.update();
        assert_eq!(
            app.world().resource::<CameraChanges>().0.last(),
            Some(&(true, true))
        );
        app.update();
        assert_eq!(
            app.world().resource::<CameraChanges>().0.last(),
            Some(&(false, false))
        );
    }

    #[derive(Resource, Default)]
    struct TargetProjectionChanges(Vec<bool>);

    fn record_target_projection_changes(
        cameras: Query<Ref<Projection>, With<Camera>>,
        mut changes: ResMut<TargetProjectionChanges>,
    ) {
        changes.0 = cameras
            .iter()
            .map(|projection| projection.is_changed())
            .collect();
    }

    #[test]
    fn paired_camera_programmatic_resize_is_synchronized_before_bevy_update() {
        use bevy::{
            render::{camera::camera_system, texture::ManualTextureViews},
            window::{WindowCreated, WindowResized, WindowScaleFactorChanged},
        };

        let (mut app, window, _, camera) = fixture((1280, 720), Vec3::new(-8.2, 0.0, 0.0));
        app.init_resource::<Assets<Image>>()
            .init_resource::<ManualTextureViews>()
            .init_resource::<TargetProjectionChanges>()
            .add_systems(Last, record_target_projection_changes)
            .add_message::<WindowCreated>()
            .add_message::<WindowResized>()
            .add_message::<WindowScaleFactorChanged>()
            .add_message::<AssetEvent<Image>>()
            .add_systems(PostUpdate, camera_system);
        app.world_mut()
            .entity_mut(camera)
            .insert((Camera::default(), RenderTarget::default()));
        let overlay = app
            .world_mut()
            .spawn((
                Camera::default(),
                RenderTarget::default(),
                Projection::from(OrthographicProjection::default_2d()),
            ))
            .id();
        app.update();
        let sizes = |app: &App| {
            [camera, overlay].map(|entity| {
                app.world()
                    .get::<Camera>(entity)
                    .unwrap()
                    .physical_target_size()
                    .unwrap()
            })
        };
        assert_eq!(sizes(&app), [UVec2::new(1280, 720); 2]);
        // Display restoration can update Window.resolution before Winit posts
        // WindowResized. A no-op 3D Projection write used to refresh only that
        // camera here, while the same-target overlay retained its old size.
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(1600.0, 900.0);
        app.update();
        assert_eq!(sizes(&app), [UVec2::new(1280, 720); 2]);
        // Actual Bevy camera_system negative: when only the 3D projection is
        // dirty, its cached target refreshes but its overlay remains stale.
        app.world_mut()
            .get_mut::<Projection>(camera)
            .unwrap()
            .set_changed();
        app.update();
        assert_eq!(sizes(&app), [UVec2::new(1600, 900), UVec2::new(1280, 720)]);
        app.add_systems(
            PostUpdate,
            sync_primary_window_camera_sizes.before(camera_system),
        );
        app.update();
        assert_eq!(sizes(&app), [UVec2::new(1600, 900); 2]);
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(1280.0, 720.0);
        app.update();
        assert_eq!(sizes(&app), [UVec2::new(1280, 720); 2]);
        // A portrait transition also changes the production vertical span;
        // synchronizing all primary-target cameras is still required.
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set(390.0, 844.0);
        app.update();
        assert_eq!(sizes(&app), [UVec2::new(390, 844); 2]);
        app.update();
        assert_eq!(
            app.world().resource::<TargetProjectionChanges>().0,
            [false, false]
        );
    }

    #[test]
    fn tiny_windows_keep_a_nonempty_safe_rect_inside_viewport() {
        for size in [Vec2::ONE, Vec2::new(40.0, 30.0), Vec2::new(200.0, 120.0)] {
            let safe = gameplay_rect(size, false);
            assert!(safe.min.cmpge(Vec2::ZERO).all());
            assert!(safe.max.cmple(size).all());
            assert!(safe.width() > 0.0 && safe.height() > 0.0);
        }
    }
}
