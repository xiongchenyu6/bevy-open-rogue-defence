//! The same consumed ingredients visibly become the crafted item; no modal or input delay.
use super::*;
use std::collections::VecDeque;

#[derive(Resource, Default)]
struct ForgeFeedback {
    initialized: bool,
    seen: [bool; 6],
    pending: VecDeque<RecipeId>,
}
#[derive(Component)]
struct ForgeNotice {
    recipe: RecipeId,
    age: f32,
    size: Vec2,
}
#[derive(Component)]
pub(super) struct ForgeShowcase {
    pub size: Vec2,
}
#[derive(Component)]
struct ForgePart {
    index: usize,
    count: usize,
    result: bool,
}
#[derive(Component)]
struct ForgeCaption;

pub(super) fn install(app: &mut App) {
    app.init_resource::<ForgeFeedback>()
        .add_systems(Update, (observe, animate, present).chain());
}

fn observe(
    crafting: Res<CraftingState>,
    session: Res<GameSession>,
    mut feedback: ResMut<ForgeFeedback>,
) {
    let acquired = RecipeId::ALL.map(|recipe| crafting.acquired(recipe));
    if feedback.initialized && !session.finished {
        for recipe in RecipeId::ALL {
            if acquired[recipe.index()] && !feedback.seen[recipe.index()] {
                feedback.pending.push_back(recipe);
            }
        }
    } else {
        feedback.pending.clear();
    }
    feedback.initialized = true;
    feedback.seen = acquired;
}

fn present(
    mut commands: Commands,
    mut state: ResMut<ForgeFeedback>,
    assets: Res<AssetServer>,
    notices: Query<(), With<ForgeNotice>>,
    showcases: Query<(Entity, &ForgeShowcase)>,
    session: Res<GameSession>,
    panel: Res<EquipmentPanelState>,
    crafting: Res<CraftingState>,
) {
    if !panel.open() || !panel.crafting_tab || session.finished {
        // Closing never traps invisible celebrations to be replayed on a future visit.
        state.pending.clear();
        return;
    }
    if !notices.is_empty() || panel.crafting_revision != crafting.revision {
        return;
    }
    let Ok((host, showcase)) = showcases.single() else {
        return;
    };
    let Some(recipe) = state.pending.pop_front() else {
        return;
    };
    let size = showcase.size;
    commands.entity(host).with_children(|host| {
        host.spawn((
            Name::new("Forging convergence feedback"),
            ForgeNotice {
                recipe,
                age: 0.,
                size,
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                width: percent(100),
                height: percent(100),
                ..default()
            },
            ZIndex(1),
            Pickable::IGNORE,
        ))
        .with_children(|panel| {
            let count = recipe.ingredients().len();
            for (index, ingredient) in recipe.ingredients().iter().enumerate() {
                panel.spawn((
                    Name::new(format!("Forging ingredient {index}")),
                    ForgePart {
                        index,
                        count,
                        result: false,
                    },
                    ingredient.image_node(&assets),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
            let mut image = recipe.image_node(&assets);
            image.color = Color::NONE;
            panel.spawn((
                Name::new("Forging result"),
                ForgePart {
                    index: 0,
                    count,
                    result: true,
                },
                image,
                Node {
                    position_type: PositionType::Absolute,
                    ..default()
                },
                Pickable::IGNORE,
            ));
            if size.x >= 100. {
                panel.spawn((
                    ForgeCaption,
                    Text::new(""),
                    TextFont {
                        font: assets.load(super::super::localization::UI_FONT_PATH).into(),
                        font_size: FontSize::Px(12.),
                        ..default()
                    },
                    TextColor(IVORY),
                    TextLayout::justify(Justify::Center),
                    Node {
                        position_type: PositionType::Absolute,
                        bottom: px(2),
                        width: percent(100),
                        ..default()
                    },
                    Pickable::IGNORE,
                ));
            }
        });
    });
}

#[allow(clippy::type_complexity)]
fn animate(
    mut commands: Commands,
    time: Res<Time<Real>>,
    language: Res<GameLanguage>,
    session: Res<GameSession>,
    mut notices: Query<(Entity, &mut ForgeNotice), Without<ForgePart>>,
    mut parts: Query<(&ForgePart, &mut Node, &mut ImageNode), Without<SelectedArt>>,
    mut original: Query<&mut ImageNode, (With<SelectedArt>, Without<ForgePart>)>,
    mut captions: Query<&mut Text, With<ForgeCaption>>,
) {
    let mut visible = false;
    for (entity, mut notice) in &mut notices {
        // The equipment panel pauses the battle, not the player's forge feedback.
        notice.age += time.delta_secs().min(0.1);
        if notice.age >= 0.95 || session.finished {
            commands.entity(entity).despawn();
            continue;
        }
        visible = true;
        let area = notice.size;
        let converge = (notice.age / 0.42).clamp(0., 1.);
        let reveal = ((notice.age - 0.30) / 0.22).clamp(0., 1.);
        for (part, mut node, mut image) in &mut parts {
            let size = if part.result {
                area.x * 0.84 * (0.7 + reveal * 0.3 + (reveal * std::f32::consts::PI).sin() * 0.12)
            } else {
                area.x / (part.count as f32 + 0.5)
            };
            let offset = if part.result {
                0.
            } else {
                (part.index as f32 - (part.count as f32 - 1.) * 0.5) * area.x / part.count as f32
                    * (1. - converge).powi(2)
            };
            node.left = px(area.x * 0.5 + offset - size * 0.5);
            node.top = px((area.y - size) * 0.5 - if area.y >= 100. { 8. } else { 0. });
            node.width = px(size);
            node.height = px(size);
            image.color =
                Color::srgba(1., 1., 1., if part.result { reveal } else { 1. - converge });
        }
        for mut caption in &mut captions {
            caption.0 = format!(
                "{}\n{}",
                language.pick("FORGED", "合成成功"),
                notice.recipe.name(*language)
            );
        }
    }
    for mut art in &mut original {
        art.color = if visible { Color::NONE } else { Color::WHITE };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forging_feedback_observes_real_acquisition_once_even_while_paused() {
        let mut app = App::new();
        app.init_resource::<CraftingState>()
            .init_resource::<GameSession>()
            .init_resource::<ForgeFeedback>()
            .add_systems(Update, observe);
        let (_, vitals, market) = crate::game::market::purchased_crafting_materials_for_test();
        app.insert_resource(vitals).insert_resource(market);
        app.update();
        app.world_mut().resource_mut::<GameSession>().paused = true;
        assert!(crate::game::crafting::craft_recipe(
            app.world_mut(),
            RecipeId::Bloodreaver
        ));
        app.update();
        app.update();
        assert_eq!(
            app.world()
                .resource::<ForgeFeedback>()
                .pending
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![RecipeId::Bloodreaver]
        );
        assert!(!crate::game::crafting::craft_recipe(
            app.world_mut(),
            RecipeId::Bloodreaver
        ));
        app.update();
        assert_eq!(app.world().resource::<ForgeFeedback>().pending.len(), 1);
    }
}
