//! Entity graph example demonstrating non-tree relationships.
//!
//! This example shows how to:
//! - Model inventory systems with entity references
//! - Handle optional relationships (targets, equipped items)
//! - Navigate complex entity graphs
//! - Use follow_opt for optional references
//! - Use `WorldExt` to create `EntityPtr` without unsafe blocks
//!
//! Run with: `cargo run --example entity_graph`

use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, EntityPtr, WorldExt};

// Character components

#[derive(Component)]
struct Name(&'static str);

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Target(Option<EntityHandle>);

#[derive(Component)]
struct Inventory(Vec<EntityHandle>);

#[derive(Component)]
struct Equipped {
    weapon: Option<EntityHandle>,
    armor: Option<EntityHandle>,
}

// Item components

#[derive(Component)]
struct Weight(f32);

#[derive(Component)]
struct Damage(i32);

#[derive(Component)]
struct Defense(i32);

// Calculate total inventory weight
fn total_weight(character: EntityPtr) -> f32 {
    character
        .get::<Inventory>()
        .map(|inv| {
            inv.0
                .iter()
                .filter_map(|h| character.follow_handle(*h).get::<Weight>())
                .map(|w| w.0)
                .sum()
        })
        .unwrap_or(0.0)
}

// Get equipped weapon damage (if any)
fn equipped_damage(character: EntityPtr) -> Option<i32> {
    character
        .get::<Equipped>()
        .and_then(|e| e.weapon)
        .and_then(|h| character.follow_handle(h).get::<Damage>())
        .map(|d| d.0)
}

// Get equipped armor defense (if any)
fn equipped_defense(character: EntityPtr) -> Option<i32> {
    character
        .get::<Equipped>()
        .and_then(|e| e.armor)
        .and_then(|h| character.follow_handle(h).get::<Defense>())
        .map(|d| d.0)
}

// Get target's name and health (if targeting someone)
fn target_info(character: EntityPtr) -> Option<(&'static str, i32)> {
    character
        .follow_opt::<Target, _>(|t| t.0)
        .and_then(|target| {
            let name = target.get::<Name>()?.0;
            let health = target.get::<Health>()?.0;
            Some((name, health))
        })
}

// Find all characters targeting a specific entity.
// Note: In a real Bevy system you'd use a Query<Entity, With<Target>> instead of a
// candidates slice. We use a slice here because this example runs in main(), not a system.
fn find_attackers(world: &World, candidates: &[Entity], target_entity: Entity) -> Vec<EntityPtr> {
    candidates
        .iter()
        .filter_map(|&entity| {
            let ptr = world.entity_ptr(entity);

            // Check if this entity has a Target component pointing to our target
            ptr.get::<Target>().and_then(|t| {
                t.0.filter(|h| h.entity() == target_entity)
                    .map(|_| ptr)
            })
        })
        .collect()
}

fn main() {
    let mut world = World::new();

    println!("Setting up game entities...\n");

    // Create some items
    let sword = world
        .spawn((Name("Iron Sword"), Weight(5.0), Damage(15)))
        .id();
    let shield = world
        .spawn((Name("Wooden Shield"), Weight(8.0), Defense(10)))
        .id();
    let potion = world.spawn((Name("Health Potion"), Weight(0.5))).id();
    let gold = world.spawn((Name("Gold Coins"), Weight(1.0))).id();
    let bow = world
        .spawn((Name("Short Bow"), Weight(3.0), Damage(10)))
        .id();

    // Create the hero
    let hero = world
        .spawn((
            Name("Hero"),
            Health(100),
            Target(None), // Not targeting anyone yet
            Inventory(vec![
                EntityHandle::new(sword),
                EntityHandle::new(shield),
                EntityHandle::new(potion),
                EntityHandle::new(gold),
            ]),
            Equipped {
                weapon: Some(EntityHandle::new(sword)),
                armor: Some(EntityHandle::new(shield)),
            },
        ))
        .id();

    // Create an enemy targeting the hero
    let goblin = world
        .spawn((
            Name("Goblin"),
            Health(30),
            Target(Some(EntityHandle::new(hero))),
            Inventory(vec![EntityHandle::new(bow)]),
            Equipped {
                weapon: Some(EntityHandle::new(bow)),
                armor: None,
            },
        ))
        .id();

    // Create another enemy also targeting the hero
    let orc = world
        .spawn((
            Name("Orc"),
            Health(50),
            Target(Some(EntityHandle::new(hero))),
        ))
        .id();

    // Demonstrate entity graph queries
    let hero_ptr = world.entity_ptr(hero);
    let goblin_ptr = world.entity_ptr(goblin);

    // 1. Calculate inventory weight
    let weight = total_weight(hero_ptr);
    println!("Hero's inventory weight: {} lbs", weight);
    println!("  Expected: 5.0 + 8.0 + 0.5 + 1.0 = 14.5");
    assert!((weight - 14.5).abs() < 0.01);

    // 2. Check equipped items
    let damage = equipped_damage(hero_ptr);
    println!("\nHero's equipped weapon damage: {:?}", damage);
    assert_eq!(damage, Some(15));

    let defense = equipped_defense(hero_ptr);
    println!("Hero's equipped armor defense: {:?}", defense);
    assert_eq!(defense, Some(10));

    // 3. Goblin's equipment
    let goblin_damage = equipped_damage(goblin_ptr);
    println!("\nGoblin's equipped weapon damage: {:?}", goblin_damage);
    assert_eq!(goblin_damage, Some(10));

    let goblin_defense = equipped_defense(goblin_ptr);
    println!("Goblin's equipped armor defense: {:?}", goblin_defense);
    assert_eq!(goblin_defense, None); // No armor equipped

    // 4. Check targeting
    let hero_target = target_info(hero_ptr);
    println!("\nHero's target: {:?}", hero_target);
    assert_eq!(hero_target, None);

    let goblin_target = target_info(goblin_ptr);
    println!("Goblin's target: {:?}", goblin_target);
    assert_eq!(goblin_target, Some(("Hero", 100)));

    // 5. Find all entities targeting the hero
    let characters = [hero, goblin, orc];
    let attackers = find_attackers(&world, &characters, hero);
    let attacker_names: Vec<_> = attackers
        .iter()
        .filter_map(|p| p.get::<Name>())
        .map(|n| n.0)
        .collect();
    println!("\nEntities targeting the hero: {:?}", attacker_names);
    assert_eq!(attacker_names.len(), 2);
    assert!(attacker_names.contains(&"Goblin"));
    assert!(attacker_names.contains(&"Orc"));

    // 6. Update hero's target to the goblin
    world
        .entity_mut(hero)
        .insert(Target(Some(EntityHandle::new(goblin))));

    // Re-query with fresh EntityPtr
    let hero_ptr = world.entity_ptr(hero);

    let hero_target = target_info(hero_ptr);
    println!("\nAfter targeting goblin:");
    println!("Hero's target: {:?}", hero_target);
    assert_eq!(hero_target, Some(("Goblin", 30)));

    println!("\nAll assertions passed!");
}
