//! Unique adventure prizes trigger real spells from completed basic attacks.
use super::bonds::{DamageClass, DamageElement, DamageProfile};
use super::combat::CombatSet;
use super::skills::{SkillRuntime, infection::Infection, schedule_typed_fixed_strike};
use super::{BossEnemy, Enemy, GameLanguage, GameSession};
use bevy::prelude::*;

pub struct BasicRelicsPlugin;
impl Plugin for BasicRelicsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BasicRelics>()
            .add_systems(Startup, setup_art)
            .add_systems(Update, (resolve, animate).chain().before(CombatSet));
    }
}
#[derive(Resource, Default, Debug)]
pub struct BasicRelics {
    owned: [bool; 3],
    counts: [u8; 3],
    pub casts_emitted: [u64; 3],
}
impl BasicRelics {
    pub fn grant(&mut self, index: u8) -> bool {
        let Some(owned) = self.owned.get_mut(usize::from(index)) else {
            return false;
        };
        if *owned {
            return false;
        }
        *owned = true;
        true
    }
    pub fn owns(&self, index: u8) -> bool {
        self.owned.get(usize::from(index)).copied().unwrap_or(false)
    }
    fn attack(&mut self) -> Vec<usize> {
        let mut casts = Vec::new();
        for (index, threshold) in [5, 4, 6].into_iter().enumerate() {
            if !self.owned[index] {
                continue;
            }
            self.counts[index] += 1;
            if self.counts[index] == threshold {
                self.counts[index] = 0;
                self.casts_emitted[index] = self.casts_emitted[index].saturating_add(1);
                casts.push(index);
            }
        }
        casts
    }
}
pub fn relic_name(index: u8, language: GameLanguage) -> &'static str {
    match index {
        0 => language.pick("Cataclysm volley", "星陨箭簇"),
        1 => language.pick("Glacial shatter", "极寒碎星"),
        _ => language.pick("Plague cascade", "瘟疫连锁"),
    }
}
pub fn relic_description(index: u8, language: GameLanguage) -> &'static str {
    match index {
        0 => language.pick("Every 5 basic attacks: 3 meteors, each deals 150% attack + 60% spell power in 3.5m and burns. Survives hero absorption.", "每5次普攻降下3颗陨星，每颗在3.5米内造成150%攻击力＋60%法术强度的火焰伤害并灼烧；吞噬换英雄后保留。"),
        1 => language.pick("Every 4 basic attacks: a 5m ice burst deals 180% attack + 120% spell power and freezes for 2s (reduced on elites/bosses). Survives absorption.", "每4次普攻触发5米冰爆，造成180%攻击力＋120%法术强度的冰伤并冻结2秒（精英／首领会缩短）；吞噬后保留。"),
        _ => language.pick("Every 6 basic attacks: infect foes within 4m for 6s. Each second deals 30% attack + 20% spell power; spreads after 2s, at most 2 generations / 32 infections. Survives absorption.", "每6次普攻感染4米内敌人，持续6秒，每秒造成30%攻击力＋20%法术强度伤害；2秒后传播，最多两代／32个感染目标；吞噬后保留。"),
    }
}
#[derive(Component)]
pub struct RelicCast {
    pub index: u8,
    pub target: Vec3,
    remaining: f32,
    damage: f32,
}
pub(crate) fn on_attack(commands: &mut Commands, target: Vec3, attack: f32, spell: f32) {
    commands.queue(move |world: &mut World| {
        let Some(mut relics) = world.get_resource_mut::<BasicRelics>() else {
            return;
        };
        let casts = relics.attack();
        let art = world
            .get_resource::<RelicArt>()
            .map(|art| (art.orb.clone(), art.colors.clone()));
        for index in casts {
            let damage = match index {
                0 => attack * 1.5 + spell * 0.6,
                1 => attack * 1.8 + spell * 1.2,
                _ => attack * 0.3 + spell * 0.2,
            };
            if index == 0 {
                for n in 0..3 {
                    let target =
                        target + Vec3::new((n as f32 - 1.0) * 2.0, 0.0, (n % 2) as f32 * 1.2);
                    let remaining = 0.45 + n as f32 * 0.2;
                    let mut entity = world.spawn((
                        Name::new("Relic meteor cast"),
                        RelicCast {
                            index: 0,
                            target,
                            remaining,
                            damage,
                        },
                        Transform::from_translation(target + Vec3::Y * remaining * 12.0)
                            .with_scale(Vec3::new(1.1, 1.8, 1.1)),
                    ));
                    if let Some((mesh, colors)) = &art {
                        entity.insert((Mesh3d(mesh.clone()), MeshMaterial3d(colors[0].clone())));
                    }
                }
            } else {
                world.spawn((
                    Name::new("Relic burst cast"),
                    RelicCast {
                        index: index as u8,
                        target,
                        remaining: 0.4,
                        damage,
                    },
                ));
            }
        }
    });
}
#[derive(Resource)]
struct RelicArt {
    shard: Handle<Mesh>,
    orb: Handle<Mesh>,
    colors: [Handle<StandardMaterial>; 3],
}
#[derive(Component)]
struct RelicMote {
    remaining: f32,
    duration: f32,
    velocity: Vec3,
    scale: Vec3,
}
fn setup_art(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let colors = [(1.0, 0.22, 0.025), (0.12, 0.8, 1.0), (0.36, 1.0, 0.12)].map(|(r, g, b)| {
        materials.add(StandardMaterial {
            base_color: Color::srgba(r, g, b, 0.55),
            emissive: LinearRgba::new(r * 5.0, g * 5.0, b * 5.0, 1.0),
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        })
    });
    commands.insert_resource(RelicArt {
        shard: meshes.add(Cone::new(0.22, 1.4)),
        orb: meshes.add(Sphere::new(0.5)),
        colors,
    });
}
#[allow(clippy::too_many_arguments)]
fn resolve(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    art: Option<Res<RelicArt>>,
    mut runtime: ResMut<SkillRuntime>,
    enemies: Query<(Entity, &Enemy, &Transform, Option<&BossEnemy>), Without<RelicCast>>,
    infected: Query<Entity, With<Infection>>,
    motes: Query<&RelicMote>,
    mut casts: Query<(Entity, &mut RelicCast, Option<&mut Transform>)>,
) {
    if session.finished {
        for (entity, ..) in &casts {
            commands.entity(entity).despawn();
        }
        return;
    }
    if session.paused {
        return;
    }
    let mut occupied: std::collections::HashSet<Entity> = infected.iter().collect();
    let mut budget = 256usize.saturating_sub(motes.iter().count());
    for (entity, mut cast, transform) in &mut casts {
        cast.remaining -= time.delta_secs();
        if let Some(mut transform) = transform {
            transform.translation = cast.target + Vec3::Y * (cast.remaining * 12.0).max(0.3);
        }
        if cast.remaining > 0.0 {
            continue;
        }
        let radius = match cast.index {
            0 => 3.5,
            1 => 5.0,
            _ => 4.0,
        };
        if cast.index < 2 {
            schedule_typed_fixed_strike(
                &mut commands,
                cast.target,
                cast.damage,
                0.0,
                radius,
                true,
                radius,
                DamageProfile::new(
                    if cast.index == 0 {
                        DamageElement::Flame
                    } else {
                        DamageElement::Frost
                    },
                    DamageClass::Spell,
                ),
            );
        }
        for (target, enemy, transform, boss) in &enemies {
            if enemy.hp <= 0.0 || transform.translation.xz().distance(cast.target.xz()) > radius {
                continue;
            }
            match cast.index {
                0 => runtime.ignite_relic(target, cast.damage * 0.4),
                1 => runtime.freeze_relic(target, enemy.speed, enemy.elite, boss.is_some(), 2.0),
                _ => {
                    if occupied.len() < 32 && occupied.insert(target) {
                        commands
                            .entity(target)
                            .insert(Infection::new(5, 0, cast.damage));
                    }
                }
            }
        }
        if let Some(art) = art.as_deref() {
            for n in 0..budget.min(48) {
                let angle = n as f32 * 2.39996;
                let direction = Vec3::new(angle.cos(), 0.0, angle.sin());
                let scale = match cast.index {
                    0 => Vec3::splat(0.4 + (n % 3) as f32 * 0.15),
                    1 => Vec3::new(0.6, 1.8, 0.6),
                    _ => Vec3::splat(0.8),
                };
                let position =
                    cast.target + direction * (n % 6) as f32 * radius / 6.0 + Vec3::Y * 0.5;
                commands.spawn((
                    Name::new(format!("Relic {} spectacle", cast.index)),
                    Mesh3d(if cast.index == 1 {
                        art.shard.clone()
                    } else {
                        art.orb.clone()
                    }),
                    MeshMaterial3d(art.colors[usize::from(cast.index)].clone()),
                    Transform::from_translation(position).with_scale(scale),
                    RelicMote {
                        remaining: 0.75,
                        duration: 0.75,
                        velocity: direction * radius
                            + Vec3::Y * (if cast.index == 0 { 3.0 } else { 0.4 }),
                        scale,
                    },
                ));
            }
            budget = budget.saturating_sub(48);
        }
        commands.entity(entity).despawn();
    }
}
fn animate(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<GameSession>,
    mut motes: Query<(Entity, &mut RelicMote, &mut Transform)>,
) {
    for (entity, mut mote, mut transform) in &mut motes {
        if session.finished {
            commands.entity(entity).despawn();
            continue;
        }
        if session.paused {
            continue;
        }
        mote.remaining -= time.delta_secs();
        if mote.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation += mote.velocity * time.delta_secs();
        transform.scale = mote.scale * (mote.remaining / mote.duration).sqrt();
    }
}
pub(crate) fn reset_run(world: &mut World) {
    world.insert_resource(BasicRelics::default());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unique_rewards_only_trigger_their_owned_attack_cycles() {
        let mut relics = BasicRelics::default();
        for _ in 0..20 {
            assert!(relics.attack().is_empty());
        }
        assert!(relics.grant(0));
        assert!(!relics.grant(0));
        assert!(!relics.grant(3));
        assert!(relics.grant(1));
        assert!(relics.grant(2));
        for _ in 0..20 {
            relics.attack();
        }
        assert_eq!(relics.casts_emitted, [4, 5, 3]);
    }
    #[test]
    fn real_attack_commands_schedule_three_delayed_meteors_and_reset_clears_ownership() {
        use bevy::ecs::system::RunSystemOnce;
        let mut world = World::new();
        world.init_resource::<BasicRelics>();
        world.resource_mut::<BasicRelics>().grant(0);
        for _ in 0..5 {
            world
                .run_system_once(|mut commands: Commands| {
                    on_attack(&mut commands, Vec3::X, 100.0, 50.0)
                })
                .unwrap();
        }
        let casts: Vec<_> = world.query::<&RelicCast>().iter(&world).collect();
        assert_eq!(casts.len(), 3);
        assert!(casts.iter().all(|cast| (cast.damage - 180.0).abs() < 0.001));
        reset_run(&mut world);
        assert!(!world.resource::<BasicRelics>().owns(0));
    }
}
