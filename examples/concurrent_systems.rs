//! Concurrent systems example demonstrating multi-threaded read-only access.
//!
//! This example shows how to:
//! - Create multiple read-only systems that use EntityPtr
//! - Understand why concurrent reads are safe with this crate
//! - Use the `WorldExt` trait for ergonomic EntityPtr creation
//!
//! Run with: `cargo run --example concurrent_systems`
//!
//! # Thread Safety Explanation
//!
//! Each system creates its own `EntityPtr` instances using `world.entity_ptr()`.
//! These types are intentionally NOT Send/Sync, meaning they cannot be shared
//! across threads. However, Bevy's scheduler can run multiple read-only systems
//! in parallel because:
//!
//! 1. Each system has its own `&World` reference (immutable borrow)
//! 2. Each system creates independent `EntityPtr` instances from that reference
//! 3. All operations through `EntityPtr` are read-only
//!
//! This is the intended usage pattern: use `world.entity_ptr()` to get pointers,
//! traverse entities within the system, and let everything drop before returning.

use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, EntityPtr, WorldExt};

// Components for our hierarchy

#[derive(Component)]
struct Name(&'static str);

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Armor(i32);

#[derive(Component)]
struct Children(Vec<EntityHandle>);

#[derive(Component)]
struct RootMarker;

// Recursive helper: sum health across a hierarchy
fn sum_health(node: EntityPtr) -> i32 {
    let my_health = node.get::<Health>().map(|h| h.0).unwrap_or(0);

    let children_health: i32 = node
        .get::<Children>()
        .map(|c| c.0.iter().map(|h| sum_health(node.follow_handle(*h))).sum())
        .unwrap_or(0);

    my_health + children_health
}

// Recursive helper: sum armor across a hierarchy
fn sum_armor(node: EntityPtr) -> i32 {
    let my_armor = node.get::<Armor>().map(|a| a.0).unwrap_or(0);

    let children_armor: i32 = node
        .get::<Children>()
        .map(|c| c.0.iter().map(|h| sum_armor(node.follow_handle(*h))).sum())
        .unwrap_or(0);

    my_armor + children_armor
}

/// System A: Computes total health across hierarchies.
///
/// This system can run concurrently with other read-only systems because:
/// - It only reads from the World (no mutations)
/// - Its EntityPtr instances are local to this system
/// - Bevy's scheduler detects the `&World` parameter and allows parallel execution
fn compute_health_system(world: &World, query: Query<(Entity, &Name), With<RootMarker>>) {
    for (entity, name) in &query {
        let root_ptr = world.entity_ptr(entity);
        let total_health = sum_health(root_ptr);
        println!("[Health System] {} total health: {}", name.0, total_health);
    }
}

/// System B: Computes total armor across hierarchies.
///
/// This system runs concurrently with compute_health_system because both
/// only perform read operations. In a real Bevy app, the scheduler would
/// automatically parallelize these systems.
fn compute_armor_system(world: &World, query: Query<(Entity, &Name), With<RootMarker>>) {
    for (entity, name) in &query {
        let root_ptr = world.entity_ptr(entity);
        let total_armor = sum_armor(root_ptr);
        println!("[Armor System] {} total armor: {}", name.0, total_armor);
    }
}

fn main() {
    let mut world = World::new();

    // Build two separate hierarchies:
    //
    // Squad Alpha:           Squad Beta:
    //   alpha (H:100, A:50)    beta (H:80, A:30)
    //    ├─ a1 (H:50, A:20)     ├─ b1 (H:40, A:15)
    //    └─ a2 (H:50, A:20)     └─ b2 (H:40, A:15)
    //
    println!("Building entity hierarchies...\n");

    // Build Squad Alpha
    let a1 = world.spawn((Name("a1"), Health(50), Armor(20))).id();
    let a2 = world.spawn((Name("a2"), Health(50), Armor(20))).id();
    let alpha = world
        .spawn((
            Name("Squad Alpha"),
            Health(100),
            Armor(50),
            RootMarker,
            Children(vec![EntityHandle::new(a1), EntityHandle::new(a2)]),
        ))
        .id();

    // Build Squad Beta
    let b1 = world.spawn((Name("b1"), Health(40), Armor(15))).id();
    let b2 = world.spawn((Name("b2"), Health(40), Armor(15))).id();
    let _beta = world
        .spawn((
            Name("Squad Beta"),
            Health(80),
            Armor(30),
            RootMarker,
            Children(vec![EntityHandle::new(b1), EntityHandle::new(b2)]),
        ))
        .id();

    // In a real Bevy application, you would add systems like this:
    //
    //   app.add_systems(Update, (compute_health_system, compute_armor_system));
    //
    // Bevy's scheduler would automatically run them in parallel because
    // both systems only have `&World` access (no exclusive/mutable access).

    println!("Simulating concurrent system execution:\n");
    println!("(In a real Bevy app, these would run in parallel)\n");

    // Create a schedule and add both systems
    let mut schedule = Schedule::default();
    schedule.add_systems((compute_health_system, compute_armor_system));

    // Run the schedule - systems execute (potentially in parallel with multi-threading)
    schedule.run(&mut world);

    // Verify the computed values
    println!("\n--- Verification ---");

    let alpha_ptr = world.entity_ptr(alpha);

    let alpha_health = sum_health(alpha_ptr);
    let alpha_armor = sum_armor(alpha_ptr);

    println!(
        "Squad Alpha: health={}, armor={}",
        alpha_health, alpha_armor
    );
    assert_eq!(alpha_health, 200); // 100 + 50 + 50
    assert_eq!(alpha_armor, 90); // 50 + 20 + 20

    println!("\nAll assertions passed!");
    println!("\nKey takeaway: Use world.entity_ptr() in each system.");
    println!("EntityPtr is NOT shared between systems - each system has");
    println!("independent instances that are safe for concurrent reads.");
}
