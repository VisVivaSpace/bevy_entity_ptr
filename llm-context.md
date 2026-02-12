# LLM Context: bevy_entity_ptr

## What This Crate Does

`bevy_entity_ptr` provides ergonomic smart-pointer-like access to Bevy ECS entities with **immutable-only** semantics. It lets you traverse entity graphs (trees, linked lists, inventories) without threading `&World` through every function call.

## When to Use This Crate

Use `bevy_entity_ptr` when you need to:
- Traverse entity graphs recursively (e.g., sum health across a tree hierarchy)
- Follow entity references stored in components without passing `&World` everywhere
- Store entity references in components safely (`EntityHandle` is `Send + Sync`)

Do **not** use this crate for mutations — it is read-only by design.

## Quick Start

Add to `Cargo.toml`:
```toml
[dependencies]
bevy_entity_ptr = "0.4"
```

### Core Types

| Type | Use Case |
|------|----------|
| `EntityHandle` | Store in components (8 bytes, Send + Sync) |
| `BoundEntity<'w>` | Scoped access with explicit `&World` lifetime |
| `EntityPtr` | Ergonomic traversal — no `&World` param needed |

### Common Patterns

```rust
use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, WorldExt, EntityPtr};

#[derive(Component)]
struct Parent(EntityHandle);

#[derive(Component)]
struct Health(i32);

// Recursive traversal — no &World parameter needed
fn find_root(node: EntityPtr) -> EntityPtr {
    match node.follow::<Parent, _>(|p| p.0) {
        Some(parent) => find_root(parent),
        None => node,
    }
}

fn my_system(world: &World, entity: Entity) {
    // WorldExt hides all unsafe — no unsafe blocks needed
    let ptr = world.entity_ptr(entity);
    if let Some(health) = ptr.get::<Health>() {
        println!("Health: {}", health.0);
    }
}
```

### Key Methods on EntityPtr

- `get::<T>()` — get a component (`Option<&T>`)
- `has::<T>()` — check if component exists
- `is_alive()` — check if entity still exists
- `follow::<T, _>(|c| c.handle)` — follow a reference component to another entity
- `follow_opt::<T, _>(|c| c.optional_handle)` — follow an optional reference
- `follow_handle(handle)` — convert an `EntityHandle` to `EntityPtr` using same world

### Safety Model

- Only one `unsafe` point: `WorldRef::new()`, hidden by `WorldExt` extension trait
- Users never write `unsafe` blocks
- `EntityPtr` is `!Send` — cannot escape the creating thread
- Stale references (despawned entities) return `None`, not UB
