//! Mixed usage example showing EntityHandle storage with EntityPtr traversal.
//!
//! This example demonstrates:
//! - Storing EntityHandle in components (Send + Sync)
//! - Using BoundEntity for explicit world parameter style
//! - Using EntityPtr for ergonomic traversal
//! - Converting between the two approaches
//! - Handling stale references gracefully
//!
//! Run with: `cargo run --example mixed_usage`

use bevy_ecs::prelude::*;
use bevy_entity_ptr::{BoundEntity, EntityHandle, EntityPtr, WorldRef};

// Components

#[derive(Component)]
struct Name(&'static str);

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct LinkedList {
    next: Option<EntityHandle>,
}

#[derive(Component)]
struct Team(Vec<EntityHandle>);

// Using BoundEntity (explicit world parameter style)
fn count_chain_length_bound(start: BoundEntity) -> usize {
    let mut count = 1;
    let mut current = start;

    while let Some(next) = current.follow_opt::<LinkedList, _>(|l| l.next) {
        count += 1;
        current = next;
    }

    count
}

// Using EntityPtr (ergonomic style)
fn count_chain_length_ptr(start: EntityPtr) -> usize {
    let mut count = 1;
    let mut current = start;

    while let Some(next) = current.follow_opt::<LinkedList, _>(|l| l.next) {
        count += 1;
        current = next;
    }

    count
}

// Calculate total team health using EntityPtr
fn team_health(leader: EntityPtr) -> i32 {
    leader
        .get::<Team>()
        .map(|team| {
            team.0
                .iter()
                .filter_map(|h| leader.follow_handle(*h).get::<Health>())
                .map(|h| h.0)
                .sum::<i32>()
        })
        .unwrap_or(0)
        + leader.get::<Health>().map(|h| h.0).unwrap_or(0)
}

// Find team member by name
fn find_team_member<'w>(leader: BoundEntity<'w>, target_name: &str) -> Option<BoundEntity<'w>> {
    leader.get::<Team>().and_then(|team| {
        team.0.iter().find_map(|h| {
            let member = h.bind(leader.world());
            if member.get::<Name>().map(|n| n.0) == Some(target_name) {
                Some(member)
            } else {
                None
            }
        })
    })
}

fn main() {
    let mut world = World::new();

    println!("=== Mixed Usage Example ===\n");

    // Build a linked list: a -> b -> c -> d
    println!("Building linked list: a -> b -> c -> d\n");

    let d = world.spawn((Name("d"), LinkedList { next: None })).id();
    let c = world
        .spawn((
            Name("c"),
            LinkedList {
                next: Some(EntityHandle::new(d)),
            },
        ))
        .id();
    let b = world
        .spawn((
            Name("b"),
            LinkedList {
                next: Some(EntityHandle::new(c)),
            },
        ))
        .id();
    let a = world
        .spawn((
            Name("a"),
            LinkedList {
                next: Some(EntityHandle::new(b)),
            },
        ))
        .id();

    // Store handle for later use (EntityHandle is Send + Sync)
    let a_handle = EntityHandle::new(a);

    // === BoundEntity approach ===
    println!("--- BoundEntity Approach (explicit &World) ---");

    let bound = a_handle.bind(&world);
    let length = count_chain_length_bound(bound);
    println!("Chain length from 'a': {}", length);
    assert_eq!(length, 4);

    // Get component with explicit world
    let name = a_handle.get::<Name>(&world).unwrap().0;
    println!("Starting node name: {}", name);

    // === EntityPtr approach ===
    println!("\n--- EntityPtr Approach (ergonomic) ---");

    // SAFETY: world outlives EntityPtr usage
    let w = unsafe { WorldRef::new(&world) };
    let ptr = w.from_handle(a_handle); // Convert handle to ptr

    let length = count_chain_length_ptr(ptr);
    println!("Chain length from 'a': {}", length);
    assert_eq!(length, 4);

    // Convert back to handle
    let back_to_handle = ptr.handle();
    assert_eq!(back_to_handle.entity(), a_handle.entity());
    println!("Round-trip handle conversion: OK");

    // === Team example ===
    println!("\n--- Team Example ---");

    // Create team members
    let alice = world.spawn((Name("Alice"), Health(100))).id();
    let bob = world.spawn((Name("Bob"), Health(80))).id();
    let charlie = world.spawn((Name("Charlie"), Health(120))).id();

    // Create team leader with team
    let leader = world
        .spawn((
            Name("Leader"),
            Health(150),
            Team(vec![
                EntityHandle::new(alice),
                EntityHandle::new(bob),
                EntityHandle::new(charlie),
            ]),
        ))
        .id();

    // Calculate total team health using EntityPtr
    let w = unsafe { WorldRef::new(&world) };
    let leader_ptr = w.entity(leader);
    let total = team_health(leader_ptr);
    println!("Total team health: {}", total);
    println!("  Expected: 150 + 100 + 80 + 120 = 450");
    assert_eq!(total, 450);

    // Find team member using BoundEntity
    let leader_bound = EntityHandle::new(leader).bind(&world);
    let bob_bound = find_team_member(leader_bound, "Bob").unwrap();
    let bob_health = bob_bound.get::<Health>().unwrap().0;
    println!("Bob's health: {}", bob_health);
    assert_eq!(bob_health, 80);

    // === Stale Reference Handling ===
    println!("\n--- Stale Reference Handling ---");

    let temp = world.spawn((Name("Temporary"), Health(50))).id();
    let temp_handle = EntityHandle::new(temp);

    // Check it's alive
    println!("Before despawn:");
    println!("  is_alive: {}", temp_handle.is_alive(&world));
    println!("  name: {:?}", temp_handle.get::<Name>(&world).map(|n| n.0));
    assert!(temp_handle.is_alive(&world));

    // Despawn the entity
    world.despawn(temp);

    // Handle gracefully returns None
    println!("After despawn:");
    println!("  is_alive: {}", temp_handle.is_alive(&world));
    println!("  name: {:?}", temp_handle.get::<Name>(&world).map(|n| n.0));
    assert!(!temp_handle.is_alive(&world));
    assert!(temp_handle.get::<Name>(&world).is_none());

    // === Mixing approaches in computation ===
    println!("\n--- Mixing Approaches ---");

    // Start with a stored handle
    let leader_handle = EntityHandle::new(leader);

    // Use BoundEntity for one part of the computation
    let bound = leader_handle.bind(&world);
    let leader_name = bound.get::<Name>().unwrap().0;

    // Switch to EntityPtr for another part
    let w = unsafe { WorldRef::new(&world) };
    let ptr = w.from_handle(leader_handle);
    let total_health = team_health(ptr);

    println!(
        "{}'s team has total health of {}",
        leader_name, total_health
    );

    // Can also convert EntityPtr back to BoundEntity's handle
    let ptr_handle = ptr.handle();
    let rebound = ptr_handle.bind(&world);
    assert_eq!(rebound.get::<Name>().unwrap().0, leader_name);
    println!("Seamless conversion between approaches: OK");

    println!("\nAll assertions passed!");
}
