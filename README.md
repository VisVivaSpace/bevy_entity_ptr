# bevy_entity_ptr

Ergonomic smart-pointer-like access to Bevy ECS entities with immutable-only semantics.

## Overview

This crate provides two complementary approaches for accessing entity data in Bevy:

| Type | Size | Use Case |
|------|------|----------|
| `EntityHandle` | 8 bytes | Store in components, explicit `&World` parameter |
| `BoundEntity<'w>` | 16 bytes | Fluent access within a scope |
| `WorldRef` | 8 bytes | System entry point for `EntityPtr` approach |
| `EntityPtr` | 16 bytes | Ergonomic traversal without `&World` parameter |

## Design Principles

- **Immutable only** - No `get_mut` variants (functional programming style)
- **Single unsafe boundary** - Only `WorldRef::new()` is unsafe
- **Graceful stale handling** - Despawned entities return `None`, not undefined behavior
- **Zero-cost where possible** - `#[repr(transparent)]`, `#[inline]`, `const fn`

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy_entity_ptr = "0.1"
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

### Ergonomic Approach: WorldRef + EntityPtr

Use this when you want fluent traversal without passing `&World` everywhere.
There is **one unsafe point**: `WorldRef::new()`.

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldRef, EntityPtr, EntityHandle};

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

fn health_system(world: &World) {
    // SAFETY: WorldRef is dropped before system returns
    let w = unsafe { WorldRef::new(world) };

    // Use EntityPtr for ergonomic traversal
    for entity in world.iter_entities() {
        let ptr = w.entity(entity.id());
        let total = sum_tree_health(ptr);
        println!("Subtree health: {}", total);
    }
}
```

### Mixed Usage

Store handles in components, use smart pointers for traversal:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, WorldRef, EntityPtr};

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
```

## Navigation Traits (Optional)

Enable the `nav-traits` feature for parent/child navigation helpers:

```toml
[dependencies]
bevy_entity_ptr = { version = "0.1", features = ["nav-traits"] }
```

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, WorldRef, HasParent, HasChildren};

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
    // SAFETY: WorldRef scoped to this function
    let w = unsafe { WorldRef::new(world) };
    let ptr = w.entity(entity);

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

## Multi-Threaded Usage Example

Multiple read-only systems can use `bevy_entity_ptr` concurrently. Each system creates its own `WorldRef` at entry, and Bevy's scheduler runs them in parallel when all systems only read:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, EntityPtr, WorldRef};

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
    // SAFETY: System has &World access, WorldRef dropped before system returns
    let world_ref = unsafe { WorldRef::new(world) };

    for entity in &query {
        let total = sum_health(world_ref.entity(entity));
        println!("Total health: {}", total);
    }
}

/// System B: Runs concurrently with System A
fn compute_armor_system(world: &World, query: Query<Entity, With<RootMarker>>) {
    // SAFETY: System has &World access, WorldRef dropped before system returns
    let world_ref = unsafe { WorldRef::new(world) };

    for entity in &query {
        let total = sum_armor(world_ref.entity(entity));
        println!("Total armor: {}", total);
    }
}

// Bevy's scheduler runs both systems in parallel - both only read
fn setup_app(app: &mut App) {
    app.add_systems(Update, (compute_health_system, compute_armor_system));
}
```

**Why this is safe:**

- Each system creates its **own** `WorldRef` instance at entry
- `WorldRef` and `EntityPtr` are **NOT** `Send`/`Sync` - they cannot be shared between threads
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

The only unsafe code is `WorldRef::new()`. The caller must ensure:

1. The `World` outlives all `EntityPtr` instances created from the `WorldRef`
2. The `World` is NOT mutated while any `EntityPtr` exists

In Bevy systems, this is naturally satisfied: systems with `&World` access cannot mutate.
Create `WorldRef` at system entry, use it for reads, and let it drop before the system returns.

## What This Crate Does NOT Support (By Design)

- **Mutable access** - Use Bevy's native APIs for mutations
- **Despawning** - Use `world.despawn()` directly
- **Component insertion/removal** - Use Bevy's native APIs
- **Cross-frame storage of `EntityPtr`** - Use `EntityHandle` or raw `Entity` for storage

## License

MIT OR Apache-2.0
