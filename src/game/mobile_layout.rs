use bevy::prelude::*;

/// Shared measurements for the compact landscape HUD and its touch controls.
///
/// Coordinates use Bevy's logical window space with the origin at the top left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudMetrics {
    pub compact: bool,
    pub safe_x: f32,
    pub safe_y: f32,
    pub joystick_size: f32,
    pub joystick_center: Vec2,
    pub drag_radius: f32,
    pub deadzone: f32,
}

/// Returns responsive HUD measurements for a logical window size.
pub fn hud_metrics(width: f32, height: f32) -> HudMetrics {
    let width = width.max(1.0);
    let height = height.max(1.0);
    let compact = width < 1_000.0 || height < 520.0;
    let (safe_x, safe_y, joystick_size) = if compact {
        (
            (width * 0.07).clamp(28.0, 56.0),
            12.0,
            (height * 0.32).clamp(112.0, 128.0),
        )
    } else {
        (22.0, 22.0, 150.0)
    };
    let joystick_center = Vec2::new(
        safe_x + joystick_size * 0.5,
        height - safe_y - joystick_size * 0.5,
    );

    HudMetrics {
        compact,
        safe_x,
        safe_y,
        joystick_size,
        joystick_center,
        drag_radius: joystick_size * 0.42,
        deadzone: (joystick_size * 0.08).max(10.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_phone_metrics_keep_the_joystick_inside_the_safe_viewport() {
        for (width, height) in [(667.0, 375.0), (844.0, 390.0)] {
            let metrics = hud_metrics(width, height);
            let half = metrics.joystick_size * 0.5;

            assert!(metrics.compact);
            assert!(metrics.joystick_center.x - half >= metrics.safe_x - 0.001);
            assert!(metrics.joystick_center.y - half >= 0.0);
            assert!(metrics.joystick_center.x + half <= width);
            assert!(metrics.joystick_center.y + half <= height - metrics.safe_y + 0.001);
            assert!(metrics.deadzone > 0.0);
            assert!(metrics.deadzone < metrics.drag_radius);
            assert!(metrics.drag_radius < half);
        }
    }

    #[test]
    fn desktop_metrics_preserve_the_existing_joystick_geometry() {
        let metrics = hud_metrics(1280.0, 720.0);

        assert!(!metrics.compact);
        assert_eq!(metrics.safe_x, 22.0);
        assert_eq!(metrics.safe_y, 22.0);
        assert_eq!(metrics.joystick_size, 150.0);
        assert_eq!(metrics.joystick_center, Vec2::new(97.0, 623.0));
    }
}
