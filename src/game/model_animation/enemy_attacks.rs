//! Enemy poses follow resolved combat actions, never inferred cooldown or gate proximity.
use super::*;
use crate::game::EnemyAttackCue;

pub(super) fn collect(
    mut cues: MessageReader<EnemyAttackCue>,
    mut players: Query<&mut CastPlayer>,
    mut targets: Local<HashMap<Entity, Vec3>>,
) {
    targets.clear();
    for cue in cues.read() {
        targets.insert(cue.source, cue.target);
    }
    if targets.is_empty() {
        return;
    }
    for mut player in &mut players {
        if let Some(target) = targets.get(&player.actor) {
            player.enemy_attack_target = Some(*target);
            player.enemy_attack_pending = true;
        }
    }
}

pub(super) fn visible_movement(displacement: Vec3, delta: f32, enemy: bool) -> bool {
    // Ignore tiny separation corrections: they are not a new walking direction.
    let threshold = if enemy {
        (delta * 0.12).max(0.002)
    } else {
        0.001
    };
    displacement.length_squared() > threshold * threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::EnemyAttackKind;

    #[test]
    fn enemy_attacks_require_real_cues_and_face_the_resolved_target() {
        let mut app = App::new();
        app.init_resource::<Time>()
            .init_resource::<GameSession>()
            .init_resource::<HeroVitals>()
            .init_resource::<crate::game::skills::SkillRuntime>()
            .init_resource::<Assets<AnimationClip>>()
            .add_message::<EnemyAttackCue>()
            .add_systems(Update, (collect, animate_cast).chain());
        let actor = app
            .world_mut()
            .spawn((
                Enemy {
                    hp: 100.,
                    max_hp: 100.,
                    speed: 0.,
                    damage: 10.,
                    xp: 0,
                    wood: 0,
                    gold: 0,
                    kill_credit: 0,
                    elite: false,
                },
                EnemyHeroThreat::ranged(),
                Transform::default(),
                Visibility::Visible,
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                AnimationPlayer::default(),
                CastPlayer {
                    actor,
                    clips: (0..4).map(AnimationNodeIndex::new).collect(),
                    source_clips: Vec::new(),
                    pose: None,
                    previous_position: None,
                    previous_health: None,
                    previous_cooldown: None,
                    reaction_remaining: 0.,
                    enemy_attack_target: None,
                    enemy_attack_pending: false,
                },
            ))
            .id();
        app.update();
        app.world_mut()
            .get_mut::<EnemyHeroThreat>(actor)
            .unwrap()
            .attack_remaining = 9.;
        app.update();
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Idle),
            "cooldown changes alone cannot invent an attack"
        );
        app.world_mut().write_message(EnemyAttackCue {
            source: actor,
            origin: Vec3::ZERO,
            target: Vec3::X * 3.,
            kind: EnemyAttackKind::Shot,
        });
        app.update();
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Attack)
        );
        assert!(
            app.world()
                .get::<Transform>(actor)
                .unwrap()
                .forward()
                .dot(Vec3::X)
                > 0.999
        );
        app.world_mut()
            .get_mut::<Transform>(actor)
            .unwrap()
            .translation
            .z = 0.0002;
        app.world_mut().get_mut::<Enemy>(actor).unwrap().hp -= 1.;
        app.update();
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Attack),
            "small incoming damage does not erase a just-started attack"
        );
        assert!(
            app.world()
                .get::<Transform>(actor)
                .unwrap()
                .forward()
                .dot(Vec3::X)
                > 0.999
        );
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f32(0.5));
        app.update();
        assert_eq!(
            app.world().get::<CastPlayer>(player).unwrap().pose,
            Some(Pose::Idle),
            "attack does not fabricate another cycle"
        );
    }

    #[test]
    fn separation_noise_does_not_reorient_enemies() {
        assert!(!visible_movement(Vec3::X * 0.0003, 1. / 60., true));
        assert!(visible_movement(Vec3::X * 0.02, 1. / 60., true));
        assert!(visible_movement(Vec3::X * 0.0015, 1. / 60., false));
    }
}
