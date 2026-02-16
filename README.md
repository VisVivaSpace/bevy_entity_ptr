# bevy_entity_ptr

[![crates.io](https://img.shields.io/crates/v/bevy_entity_ptr.svg)](https://crates.io/crates/bevy_entity_ptr)
[![docs.rs](https://docs.rs/bevy_entity_ptr/badge.svg)](https://docs.rs/bevy_entity_ptr)
[![CI](https://github.com/VisVivaSpace/bevy_entity_ptr/actions/workflows/ci.yml/badge.svg)](https://github.com/VisVivaSpace/bevy_entity_ptr/actions/workflows/ci.yml)

Smart-pointer-like access to entities in [bevy_ecs](https://crates.io/crates/bevy_ecs), a high-performance Entity Component System library. Immutable only, by design.

## Why This Crate?

When working with entity relationships in ECS (parent/child hierarchies, linked structures, graphs), accessing related entities requires repeatedly passing `&World` through every function call. This crate provides two approaches that make entity traversal ergonomic:

| Type | Safety | Ergonomics | Use When |
|------|--------|------------|----------|
| `EntityPtr` | Safe API* | No lifetime params | Graph traversal, recursion, deep chains |
| `BoundEntity<'w>` | Fully safe | Scoped lifetime | Simple access, compiler-checked lifetimes |
| `EntityHandle` | Fully safe | Explicit world param | Store in components |

\*One internal `unsafe` hidden by the `WorldExt` extension trait — see [Safety](#safety).

## Installation

```toml
[dependencies]
bevy_entity_ptr = "0.6"
```

## Quick Start

Import the `WorldExt` trait and use `world.entity_ptr()` — no `unsafe` needed:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldExt, EntityPtr, EntityHandle};

#[derive(Component)]
struct Manager(EntityHandle);

#[derive(Component)]
struct Label(&'static str);

// Follow a reference to a related entity — no &World parameter needed
fn get_manager_label(employee: EntityPtr) -> Option<&'static str> {
    employee
        .follow::<Manager, _>(|m| m.0)?
        .get::<Label>()
        .map(|l| l.0)
}

// Usage: world.entity_ptr() creates an EntityPtr from any &World context
fn example(world: &World, entity: Entity) {
    let ptr = world.entity_ptr(entity);
    if let Some(label) = get_manager_label(ptr) {
        println!("Manager: {}", label);
    }
}
```

### Recursive Traversal

`EntityPtr` carries its world reference internally, so recursive functions don't need a `&World` parameter:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldExt, EntityPtr, EntityHandle};

#[derive(Component)]
struct ParentRef(EntityHandle);

#[derive(Component)]
struct Children(Vec<EntityHandle>);

#[derive(Component)]
struct Size(f64);

// Find the root of a hierarchy — no &World parameter needed
fn find_root(node: EntityPtr) -> EntityPtr {
    match node.follow::<ParentRef, _>(|p| p.0) {
        Some(parent) => find_root(parent),
        None => node,
    }
}

// Sum a value across an entire subtree
fn subtree_size(node: EntityPtr) -> f64 {
    let my_size = node.get::<Size>().map(|s| s.0).unwrap_or(0.0);
    let children_size: f64 = node
        .get::<Children>()
        .map(|c| {
            c.0.iter()
                .map(|h| subtree_size(node.follow_handle(*h)))
                .sum()
        })
        .unwrap_or(0.0);
    my_size + children_size
}
```

### Optional References

Use `follow_opt` when a reference component might be `None`:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityPtr, EntityHandle};

#[derive(Component)]
struct Supervisor(Option<EntityHandle>);

#[derive(Component)]
struct Label(&'static str);

fn get_supervisor_label(employee: EntityPtr) -> Option<&'static str> {
    employee
        .follow_opt::<Supervisor, _>(|s| s.0)?
        .get::<Label>()
        .map(|l| l.0)
}
```

## Safety

The `WorldExt::entity_ptr()` method internally transmutes `&World` to `&'static World` so that `EntityPtr` can carry the world reference without a lifetime parameter. This is what makes the ergonomic API possible — but because the `'static` lifetime is fabricated, the compiler **cannot** catch use-after-free on the world reference.

**Sound within ECS systems**: When called from a system with `&World` access, the world is guaranteed to outlive the system scope. `EntityPtr` is `!Send`, preventing escape to other threads. All operations are read-only.

**Not sound in arbitrary code**: Because `EntityPtr` holds a `'static` reference internally, the compiler won't prevent you from using it after the `World` is dropped. This would be undefined behavior.

```rust
// GOOD: EntityPtr used within a function that borrows &World
fn process_entities(world: &World, entities: &[Entity]) {
    for &entity in entities {
        let ptr = world.entity_ptr(entity);
        // ... use ptr ...
    }  // ptr dropped before &World borrow ends
}

// BAD: Do NOT do this — the 'static lifetime means the compiler won't stop you
fn bad_example() {
    let mut world = World::new();
    let entity = world.spawn(()).id();
    let ptr = world.entity_ptr(entity);
    drop(world);    // World dropped — but ptr still holds a 'static reference!
    // ptr.get::<T>();  // undefined behavior — dangling 'static reference
}
```

**For fully safe code** with no soundness caveats, use `EntityHandle` and `BoundEntity<'w>` — they carry proper lifetime parameters and are checked by the compiler.

## Fully Safe Alternative: EntityHandle + BoundEntity

If you prefer zero `unsafe` with compiler-verified lifetimes:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, BoundEntity};

#[derive(Component)]
struct ParentRef(EntityHandle);

#[derive(Component)]
struct Label(&'static str);

fn find_parent_label<'w>(entity: Entity, world: &'w World) -> Option<&'w str> {
    let bound = EntityHandle::new(entity).bind(world);
    let parent = bound.follow::<ParentRef, _>(|p| p.0)?;
    parent.get::<Label>().map(|l| l.0)
}
```

`EntityHandle` is `Send + Sync`, making it safe to store in components.

### Mixed Usage

Store handles in components, convert to smart pointers for traversal:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, WorldExt, EntityPtr};

// EntityHandle is Send + Sync — safe to store in components
#[derive(Component)]
struct Related {
    items: Vec<EntityHandle>,
}

#[derive(Component)]
struct Weight(f32);

fn total_weight(node: EntityPtr) -> f32 {
    node.get::<Related>()
        .map(|rel| {
            rel.items
                .iter()
                .filter_map(|h| node.follow_handle(*h).get::<Weight>())
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
bevy_entity_ptr = { version = "0.6", features = ["nav-traits"] }
```

Implement the traits on your components:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldExt, EntityHandle, HasParent, HasChildren};

#[derive(Component)]
struct ParentRef(Option<EntityHandle>);

impl HasParent for ParentRef {
    fn parent_handle(&self) -> Option<EntityHandle> {
        self.0
    }
}

#[derive(Component)]
struct ChildRefs(Vec<EntityHandle>);

impl HasChildren for ChildRefs {
    fn children_handles(&self) -> &[EntityHandle] {
        &self.0
    }
}

fn navigate(world: &World, entity: Entity) {
    let ptr = world.entity_ptr(entity);

    // Navigate to parent
    if let Some(parent) = ptr.nav().parent::<ParentRef>() {
        println!("Has parent: {:?}", parent.entity());
    }

    // Iterate children (returns an iterator, zero allocation)
    let child_count = ptr.nav_many().children::<ChildRefs>().count();
    println!("Has {} children", child_count);
}
```

## Thread Safety

| Type | Send | Sync | Notes |
|------|------|------|-------|
| `EntityHandle` | Yes | Yes | Safe to store in components |
| `BoundEntity<'w>` | No | No | Borrows `&World` |
| `WorldRef` | No | No | System-scoped only |
| `EntityPtr` | No | No | System-scoped only |

Multiple read-only systems can use `bevy_entity_ptr` concurrently — the scheduler runs them in parallel when all systems only read.

## Using EntityPtr in Collections

`EntityPtr` implements `Eq` and `Hash` (comparing entity ID only), enabling use in `HashSet` and `HashMap`:

```rust
use std::collections::HashSet;
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{WorldExt, EntityPtr};

fn collect_unique(world: &World, entities: &[Entity]) -> HashSet<Entity> {
    let mut seen = HashSet::new();
    for &entity in entities {
        seen.insert(world.entity_ptr(entity));
    }
    seen.into_iter().map(|ptr| ptr.entity()).collect()
}
```

## Stale Reference Handling

Both approaches gracefully handle despawned entities — returning `None` instead of undefined behavior:

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::EntityHandle;

#[derive(Component)]
struct Label(&'static str);

fn stale_handling(world: &mut World) {
    let entity = world.spawn(Label("temporary")).id();
    let handle = EntityHandle::new(entity);

    assert!(handle.is_alive(world));
    assert_eq!(handle.get::<Label>(world).unwrap().0, "temporary");

    world.despawn(entity);

    // Gracefully returns None — no undefined behavior
    assert!(!handle.is_alive(world));
    assert!(handle.get::<Label>(world).is_none());
}
```

## What This Crate Does NOT Support (By Design)

- **Mutable access** — Use the ECS's native APIs for mutations
- **Despawning** — Use `world.despawn()` directly
- **Component insertion/removal** — Use the ECS's native APIs
- **Cross-scope storage of `EntityPtr`** — Use `EntityHandle` or raw `Entity` for storage

## Bevy Compatibility

| `bevy_entity_ptr` | Bevy |
|--------------------|------|
| 0.6                | 0.18 |
| 0.5                | 0.18 |
| 0.4                | 0.17 |
| 0.3                | 0.16 |
| 0.2                | 0.15 |
| 0.1                | 0.15 |

## Development

This crate is co-developed with [Claude Code](https://claude.ai/code).

## License

MIT
