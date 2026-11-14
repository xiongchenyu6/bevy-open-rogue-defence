//! Equipment changes the held weapon and real basic projectiles, never their gameplay data.
use super::*;
use crate::game::{HeroVisual, Projectile, attack_vfx::WeaponFlight};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Finish {
    Steel,
    Arcane,
    Ember,
    Storm,
    Blood,
    Astral,
}
impl Finish {
    fn color(self) -> Color {
        match self {
            Self::Steel => Color::srgb(0.72, 0.84, 0.94),
            Self::Arcane => Color::srgb(0.42, 0.75, 1.0),
            Self::Ember => Color::srgb(1.0, 0.48, 0.12),
            Self::Storm => Color::srgb(0.40, 0.92, 1.0),
            Self::Blood => Color::srgb(1.0, 0.22, 0.32),
            Self::Astral => Color::srgb(0.77, 0.48, 1.0),
        }
    }
}

fn finish(equipment: &MarketEffects, crafting: &CraftingState) -> Option<Finish> {
    // Acquired is intentional: forged materials retain their combat effects.
    if crafting.acquired(RecipeId::WorldEngine) {
        Some(Finish::Astral)
    } else if crafting.acquired(RecipeId::NightCrown)
        || crafting.acquired(RecipeId::Bloodreaver)
        || equipment.equipment().contains(&MarketItem::BloodContract)
    {
        Some(Finish::Blood)
    } else if crafting.acquired(RecipeId::Suncoil)
        || equipment.equipment().contains(&MarketItem::EmberHeart)
    {
        Some(Finish::Ember)
    } else if equipment.equipment().contains(&MarketItem::StormCoil) {
        Some(Finish::Storm)
    } else if crafting.acquired(RecipeId::AstralPrism)
        || equipment
            .equipment()
            .iter()
            .any(|i| matches!(i, MarketItem::ArcLens | MarketItem::EchoCore))
    {
        Some(Finish::Arcane)
    } else if equipment.equipment().contains(&MarketItem::SerratedDisc) {
        Some(Finish::Steel)
    } else {
        None
    }
}

#[derive(Component)]
struct WeaponFinish {
    original: Handle<StandardMaterial>,
    finish: Finish,
}
#[derive(Resource, Default)]
struct FinishCache(HashMap<(AssetId<StandardMaterial>, Finish), Handle<StandardMaterial>>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<FinishCache>()
        .add_systems(PostUpdate, apply);
}

fn material(
    original: &Handle<StandardMaterial>,
    finish: Finish,
    materials: &mut Assets<StandardMaterial>,
    cache: &mut FinishCache,
) -> Option<Handle<StandardMaterial>> {
    let key = (original.id(), finish);
    if let Some(handle) = cache.0.get(&key) {
        return Some(handle.clone());
    }
    let mut value = materials.get(original)?.clone();
    // Keep authored wood/metal texture and roughness. A restrained edge sheen
    // communicates an upgrade without turning a level-one bow into a laser.
    value.base_color = value.base_color.mix(&finish.color(), 0.24);
    let tint = finish.color().to_linear();
    value.emissive += LinearRgba::new(tint.red * 0.42, tint.green * 0.42, tint.blue * 0.42, 0.);
    let handle = materials.add(value);
    cache.0.insert(key, handle.clone());
    Some(handle)
}

fn hero_weapon(
    entity: Entity,
    nodes: &Query<(Option<&Name>, Option<&ChildOf>, Has<HeroVisual>)>,
) -> bool {
    let mut cursor = Some(entity);
    let mut weapon = false;
    for _ in 0..32 {
        let Some(entity) = cursor else {
            break;
        };
        let Ok((name, parent, hero)) = nodes.get(entity) else {
            break;
        };
        weapon |= name.is_some_and(|name| {
            let name = name.as_str();
            name.starts_with("1H_") || name.starts_with("2H_") || name.contains("Weapon")
        });
        if hero {
            return weapon;
        }
        cursor = parent.map(ChildOf::parent);
    }
    false
}

#[allow(clippy::type_complexity)]
fn apply(
    mut commands: Commands,
    equipment: Res<MarketEffects>,
    crafting: Res<CraftingState>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut cache: ResMut<FinishCache>,
    nodes: Query<(Option<&Name>, Option<&ChildOf>, Has<HeroVisual>)>,
    mut meshes: Query<
        (
            Entity,
            &mut MeshMaterial3d<StandardMaterial>,
            Option<&WeaponFinish>,
            Has<Projectile>,
            Has<WeaponFlight>,
        ),
        With<Mesh3d>,
    >,
) {
    let next = finish(&equipment, &crafting);
    if next.is_none() && cache.0.is_empty() {
        return;
    }
    for (entity, mut handle, previous, projectile, basic) in &mut meshes {
        let held = hero_weapon(entity, &nodes);
        if !held && !basic {
            // A pooled projectile is hidden while retired; discard its marker so
            // a later skill projectile can never inherit a basic-weapon finish.
            if previous.is_some() {
                commands.entity(entity).remove::<WeaponFinish>();
            }
            continue;
        }
        if projectile && !basic {
            continue;
        }
        let Some(next) = next else {
            if let Some(previous) = previous {
                handle.0 = previous.original.clone();
                commands.entity(entity).remove::<WeaponFinish>();
            }
            continue;
        };
        let already_finished =
            previous.is_some_and(|p| cache.0.get(&(p.original.id(), p.finish)) == Some(&handle.0));
        if already_finished && previous.is_some_and(|p| p.finish == next) {
            continue;
        }
        // Pool reuse may assign a new base handle before this system observes retirement.
        let original = if already_finished {
            previous.unwrap().original.clone()
        } else {
            handle.0.clone()
        };
        if let Some(finished) = material(&original, next, &mut materials, &mut cache) {
            handle.0 = finished;
            commands.entity(entity).insert(WeaponFinish {
                original,
                finish: next,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn equipment_finish_is_absent_at_start_and_retained_after_material_consumption() {
        let mut crafting = CraftingState::default();
        assert_eq!(finish(&MarketEffects::default(), &crafting), None);
        let (_, mut vitals, mut effects) =
            crate::game::market::purchased_crafting_materials_for_test();
        assert_eq!(finish(&effects, &crafting), Some(Finish::Blood));
        // Use production transaction, which consumes the owned ingredient entries.
        let mut world = World::new();
        world.insert_resource(crafting);
        world.insert_resource(vitals);
        world.insert_resource(effects);
        assert!(crate::game::crafting::craft_recipe(
            &mut world,
            RecipeId::Bloodreaver
        ));
        crafting = world.remove_resource().unwrap();
        vitals = world.remove_resource().unwrap();
        effects = world.remove_resource().unwrap();
        assert_eq!(finish(&effects, &crafting), Some(Finish::Blood));
        assert!(vitals.alive);
    }
    #[test]
    fn finish_uses_shared_copy_and_preserves_authored_source() {
        let mut materials = Assets::<StandardMaterial>::default();
        let source = materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.3, 0.4),
            perceptual_roughness: 0.83,
            emissive: LinearRgba::new(3.0, 2.0, 1.0, 1.0),
            ..default()
        });
        let mut cache = FinishCache::default();
        let a = material(&source, Finish::Ember, &mut materials, &mut cache).unwrap();
        let b = material(&source, Finish::Ember, &mut materials, &mut cache).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, source);
        assert_eq!(
            materials.get(&source).unwrap().base_color,
            Color::srgb(0.2, 0.3, 0.4)
        );
        assert_eq!(materials.get(&a).unwrap().perceptual_roughness, 0.83);
        assert!(
            materials.get(&a).unwrap().emissive.red > 3.0,
            "equipment never dims a high-rank original projectile"
        );
    }
}
