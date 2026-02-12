# bevy_entity_ptr

Ergonomic smart-pointer-like access to Bevy ECS entities with immutable-only semantics.

## Overview

This crate provides two complementary approaches for accessing entity data in Bevy:

| Type | Safety | Ergonomics | Use When |
|------|--------|------------|----------|
| `EntityHandle` | ✅ Fully safe | Explicit world param | Store in components |
| `BoundEntity<'w>` | ✅ Fully safe | Scoped lifetime | Simple access, compiler-checked |
| `EntityPtr` | ✅ Safe API* | No lifetime params | Tree/graph traversal, recursion |

*One internal unsafe hidden by `WorldExt` extension trait

**Recommendation:** Start with `BoundEntity<'w>`. Use `EntityPtr` when lifetime
annotations become cumbersome for complex traversal.

## Design Principles

- **Immutable only** - No `get_mut` variants (functional programming style)
- **Safe by default** - `WorldExt` trait hides the internal unsafe, users never write `unsafe` blocks
- **Graceful stale handling** - Despawned entities return `None`, not undefined behavior
- **Zero-cost where possible** - `#[repr(transparent)]`, `#[inline]`, `const fn`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_entity_ptr = "0.5"
```

## Quick Start

### Safe Approach: EntityHandle + BoundEntity

Use this when you want fully safe code with explicit world parameters:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, BoundEntity};

#[derive(Component)]
struct Parent(EntityHandle);

#[derive(Component)]
struct Name(String);

fn find_parent_name(entity: Entity, world: &World) -> Option<String> {
    let handle = EntityHandle::new(entity);
    let bound = handle.bind(world);

    // Follow the Parent component to get the parent entity
    let parent = bound.follow::<Parent, _>(|p| p.0)?;

    // Get the parent's name
    parent.get::<Name>().map(|n| n.0.clone())
}
```

### Ergonomic Approach: WorldExt + EntityPtr

Use this when you want fluent traversal without passing `&World` everywhere.
The `WorldExt` extension trait hides the internal unsafe, so you never need to write `unsafe` blocks.

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldExt, EntityPtr, EntityHandle};

#[derive(Component)]
struct Parent(EntityHandle);

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct TreeChildren(Vec<EntityHandle>);

// Recursive tree traversal - no &World parameter needed!
fn sum_tree_health(node: EntityPtr) -> i32 {
    let my_health = node.get::<Health>().map(|h| h.0).unwrap_or(0);

    let children_health: i32 = node
        .get::<TreeChildren>()
        .map(|c| {
            c.0.iter()
                .map(|h| sum_tree_health(node.follow_handle(*h)))
                .sum()
        })
        .unwrap_or(0);

    my_health + children_health
}

// Find the root by traversing parents
fn find_root(node: EntityPtr) -> EntityPtr {
    match node.follow::<Parent, _>(|p| p.0) {
        Some(parent) => find_root(parent),
        None => node,
    }
}

fn health_system(world: &World, query: Query<Entity, With<TreeChildren>>) {
    // No unsafe needed! WorldExt provides ergonomic access
    for entity in &query {
        let ptr = world.entity_ptr(entity);
        let total = sum_tree_health(ptr);
        println!("Subtree health: {}", total);
    }
}
```

### Mixed Usage

Store handles in components, use smart pointers for traversal:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, WorldExt, EntityPtr};

// EntityHandle is Send + Sync, safe to store in components
#[derive(Component)]
struct Inventory {
    items: Vec<EntityHandle>,
}

#[derive(Component)]
struct Weight(f32);

fn total_inventory_weight(player: EntityPtr) -> f32 {
    player
        .get::<Inventory>()
        .map(|inv| {
            inv.items
                .iter()
                .filter_map(|h| player.follow_handle(*h).get::<Weight>())
                .map(|w| w.0)
                .sum()
        })
        .unwrap_or(0.0)
}

fn inventory_system(world: &World, player_entity: Entity) {
    let player = world.entity_ptr(player_entity);
    let weight = total_inventory_weight(player);
    println!("Total inventory weight: {}", weight);
}
```

## Navigation Traits (Optional)

Enable the `nav-traits` feature for parent/child navigation helpers:

```toml
[dependencies]
bevy_entity_ptr = { version = "0.5", features = ["nav-traits"] }
```

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldExt, HasParent, HasChildren};

#[derive(Component)]
struct ParentRef(Option<Entity>);

impl HasParent for ParentRef {
    fn parent_entity(&self) -> Option<Entity> {
        self.0
    }
}

#[derive(Component)]
struct ChildRefs(Vec<Entity>);

impl HasChildren for ChildRefs {
    fn children_entities(&self) -> &[Entity] {
        &self.0
    }
}

fn navigate_tree(world: &World, entity: Entity) {
    let ptr = world.entity_ptr(entity);

    // Navigate to parent
    if let Some(parent) = ptr.nav().parent::<ParentRef>() {
        println!("Has parent: {:?}", parent.entity());
    }

    // Navigate to children
    let children = ptr.nav_many().children::<ChildRefs>();
    println!("Has {} children", children.len());
}
```

## Thread Safety

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `EntityHandle` | Yes | Yes | Safe to store in components |
| `BoundEntity<'w>` | No | No | Borrows `&World` |
| `WorldRef` | No | No | System-scoped only |
| `EntityPtr` | No | No | System-scoped only |

## Using EntityPtr in Collections

`EntityPtr` implements `Eq` and `Hash`, allowing use in `HashSet` and `HashMap`:

```rust
use std::collections::HashSet;
use bevy_ecs::prelude::*;
use bevy_entity_ptr::WorldExt;

fn find_unique_targets(world: &World, entities: &[Entity]) -> HashSet<Entity> {
    let mut seen = HashSet::new();

    for &entity in entities {
        let ptr = world.entity_ptr(entity);
        if seen.insert(ptr) {
            // First time seeing this entity
        }
    }

    // Convert back to Entity for storage
    seen.into_iter().map(|ptr| ptr.entity()).collect()
}
```

**Note:** `EntityPtr` comparison uses entity ID only, assuming same-world context (the typical usage pattern).

## Multi-Threaded Usage Example

Multiple read-only systems can use `bevy_entity_ptr` concurrently. Bevy's scheduler runs them in parallel when all systems only read:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, EntityPtr, WorldExt};

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Armor(i32);

#[derive(Component)]
struct Children(Vec<EntityHandle>);

#[derive(Component)]
struct RootMarker;

fn sum_health(node: EntityPtr) -> i32 {
    let my_health = node.get::<Health>().map(|h| h.0).unwrap_or(0);
    let children_health: i32 = node
        .get::<Children>()
        .map(|c| c.0.iter().map(|h| sum_health(node.follow_handle(*h))).sum())
        .unwrap_or(0);
    my_health + children_health
}

fn sum_armor(node: EntityPtr) -> i32 {
    let my_armor = node.get::<Armor>().map(|a| a.0).unwrap_or(0);
    let children_armor: i32 = node
        .get::<Children>()
        .map(|c| c.0.iter().map(|h| sum_armor(node.follow_handle(*h))).sum())
        .unwrap_or(0);
    my_armor + children_armor
}

/// System A: Computes total health across hierarchies
fn compute_health_system(world: &World, query: Query<Entity, With<RootMarker>>) {
    for entity in &query {
        let total = sum_health(world.entity_ptr(entity));
        println!("Total health: {}", total);
    }
}

/// System B: Runs concurrently with System A
fn compute_armor_system(world: &World, query: Query<Entity, With<RootMarker>>) {
    for entity in &query {
        let total = sum_armor(world.entity_ptr(entity));
        println!("Total armor: {}", total);
    }
}

// Bevy's scheduler runs both systems in parallel - both only read
fn setup_app(app: &mut App) {
    app.add_systems(Update, (compute_health_system, compute_armor_system));
}
```

**Why this is safe:**

- `WorldExt::entity_ptr()` hides the internal unsafe - you never write `unsafe` blocks
- `EntityPtr` is **NOT** `Send`/`Sync` - it cannot escape to other threads
- Bevy's scheduler detects that both systems only have `&World` access and runs them in parallel
- All operations through `EntityPtr` are read-only by design

See `examples/concurrent_systems.rs` for a complete runnable example.

## Stale Reference Handling

Both approaches gracefully handle despawned entities:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::EntityHandle;

#[derive(Component)]
struct Name(&'static str);

fn stale_handling_example(world: &mut World) {
    let entity = world.spawn(Name("temporary")).id();
    let handle = EntityHandle::new(entity);

    // Works fine
    assert!(handle.is_alive(world));
    assert_eq!(handle.get::<Name>(world).unwrap().0, "temporary");

    // Despawn the entity
    world.despawn(entity);

    // Gracefully returns None - no undefined behavior!
    assert!(!handle.is_alive(world));
    assert!(handle.get::<Name>(world).is_none());
}
```

## Safety

**For most users:** The `WorldExt` extension trait (`world.entity_ptr(entity)`) hides all unsafe code. You never need to write `unsafe` blocks.

**For advanced users:** If you need direct access to `WorldRef::new()`, the caller must ensure:

1. The `World` outlives all `EntityPtr` instances created from the `WorldRef`
2. The `World` is NOT mutated while any `EntityPtr` exists

In Bevy systems, this is naturally satisfied: systems with `&World` access cannot mutate.

## What This Crate Does NOT Support (By Design)

- **Mutable access** - Use Bevy's native APIs for mutations
- **Despawning** - Use `world.despawn()` directly
- **Component insertion/removal** - Use Bevy's native APIs
- **Cross-frame storage of `EntityPtr`** - Use `EntityHandle` or raw `Entity` for storage

## Bevy Compatibility

| `bevy_entity_ptr` | Bevy |
|--------------------|------|
| 0.5                | 0.18 |
| 0.4                | 0.17 |
| 0.3                | 0.16 |
| 0.2                | 0.15 |
| 0.1                | 0.15 |

## Development

This crate is co-developed with [Claude Code](https://claude.ai/code).

## License

MIT
