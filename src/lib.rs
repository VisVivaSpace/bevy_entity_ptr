//! Ergonomic smart-pointer-like access to Bevy ECS entities.
//!
//! This crate provides two complementary approaches for accessing entity data
//! with immutable-only semantics (functional programming style):
//!
//! ## Type Hierarchy
//!
//! ```text
//! Entity (Bevy's raw type)
//!     │
//!     ├── EntityHandle (newtype, safe, explicit world param)
//!     │       └── .bind(world) → BoundEntity<'w> (scoped access)
//!     │
//!     └── WorldRef::entity() → EntityPtr (smart pointer, 'static world)
//! ```
//!
//! ## Choosing Between Types
//!
//! | Type | Safety | Ergonomics | Use When |
//! |------|--------|------------|----------|
//! | `EntityHandle` | ✅ Fully safe | Explicit world param | Store in components |
//! | `BoundEntity<'w>` | ✅ Fully safe | Scoped lifetime | Simple access, compiler-checked |
//! | `EntityPtr` | ✅ Safe API* | No lifetime params | Tree/graph traversal, recursion |
//!
//! *One internal unsafe hidden by `WorldExt` extension trait
//!
//! **Recommendation:** Start with `BoundEntity<'w>`. Use `EntityPtr` when lifetime
//! annotations become cumbersome for complex traversal.
//!
//! ## Safe Approach: EntityHandle + BoundEntity
//!
//! Use this when you want fully safe code with explicit world parameters:
//!
//! ```
//! use bevy_ecs::prelude::*;
//! use bevy_entity_ptr::{EntityHandle, BoundEntity};
//!
//! #[derive(Component)]
//! struct Target(EntityHandle);
//!
//! #[derive(Component)]
//! struct Name(&'static str);
//!
//! fn my_system(world: &World, query: &Query<&Target>) {
//!     // EntityHandle stores compactly in components
//!     for target in query.iter() {
//!         // Bind to world for fluent access
//!         let bound = target.0.bind(world);
//!         if let Some(name) = bound.get::<Name>() {
//!             println!("Target: {}", name.0);
//!         }
//!     }
//! }
//! ```
//!
//! ## Ergonomic Approach: WorldExt + EntityPtr
//!
//! Use this when you want fluent traversal without passing `&World` everywhere.
//! The `WorldExt` trait hides the internal unsafe, providing a clean API.
//!
//! ```
//! use bevy_ecs::prelude::*;
//! use bevy_entity_ptr::{WorldExt, EntityHandle};
//!
//! #[derive(Component)]
//! struct Target(EntityHandle);
//!
//! #[derive(Component)]
//! struct Name(&'static str);
//!
//! fn traverse_system(world: &World, query: Query<Entity, With<Target>>) {
//!     // No unsafe needed! WorldExt provides ergonomic access
//!     for entity in &query {
//!         let ptr = world.entity_ptr(entity);
//!
//!         // Follow references fluently
//!         if let Some(target) = ptr.follow::<Target, _>(|t| t.0) {
//!             if let Some(name) = target.get::<Name>() {
//!                 println!("Target: {}", name.0);
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ## Thread Safety
//!
//! | Type | Send | Sync | Notes |
//! |------|------|------|-------|
//! | `EntityHandle` | Yes | Yes | Safe to store in components |
//! | `BoundEntity` | No | No | Borrows `&World` |
//! | `WorldRef` | No | No | System-scoped only |
//! | `EntityPtr` | No | No | System-scoped only |
//!
//! ## Feature Flags
//!
//! - `nav-traits`: Enables `HasParent` and `HasChildren` traits for parent/child navigation
//!
//! ## Design Principles
//!
//! 1. **Immutable only** - No `get_mut` variants (functional style)
//! 2. **Single unsafe boundary** - Only `WorldRef::new()` is unsafe
//! 3. **Graceful stale handling** - Despawned entities return `None`, not UB
//! 4. **Zero-cost where possible** - `#[repr(transparent)]`, `#[inline]`, `const fn`
//!
//! ## Safety
//!
//! The [`WorldExt::entity_ptr()`] method internally uses `unsafe` to erase the
//! lifetime of the `&World` reference, enabling ergonomic traversal without
//! threading a lifetime parameter through every function call.
//!
//! **This is sound within Bevy systems** because:
//! - `&World` is guaranteed to outlive the system scope
//! - `EntityPtr` is `!Send`, so it cannot escape to other threads
//! - The World cannot be mutated while a system holds `&World`
//!
//! **This is NOT sound in arbitrary code** where a `World` could be dropped
//! while `EntityPtr` instances still exist. If you use `WorldExt::entity_ptr()`
//! outside of a Bevy system, you must ensure the `World` outlives all
//! `EntityPtr` instances created from it.
//!
//! For fully safe code with no soundness caveats, use [`EntityHandle`] and
//! [`BoundEntity`] instead — they carry proper lifetime parameters.

mod handle;
mod ptr;

#[cfg(feature = "nav-traits")]
mod nav;

// Core types - always available
pub use handle::{BoundEntity, BoundEntityNav, EntityHandle};
pub use ptr::{EntityPtr, EntityPtrNav, EntityPtrNavMany, WorldRef};

// Navigation traits - feature-gated
#[cfg(feature = "nav-traits")]
pub use nav::{HasChildren, HasParent};

use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

/// Extension trait for `World` providing ergonomic entity access methods.
///
/// This trait adds convenience methods to `World` that hide the internal
/// unsafe boundary, making entity access more ergonomic.
///
/// # Example
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_entity_ptr::WorldExt;
///
/// #[derive(Component)]
/// struct Health(i32);
///
/// fn my_system(world: &World, entity: Entity) {
///     // No unsafe needed!
///     let ptr = world.entity_ptr(entity);
///     if let Some(health) = ptr.get::<Health>() {
///         println!("Health: {}", health.0);
///     }
/// }
/// ```
pub trait WorldExt {
    /// Creates a `BoundEntity` for scoped access with explicit lifetime.
    ///
    /// This is the safest approach with compiler-checked lifetimes.
    /// Use when you want explicit lifetime tracking.
    fn bind_entity(&self, entity: Entity) -> BoundEntity<'_>;

    /// Creates an `EntityPtr` for ergonomic traversal.
    ///
    /// This hides the internal unsafe, providing a clean API for
    /// complex entity graph traversal. The `EntityPtr` is `!Send`,
    /// preventing escape to other threads.
    ///
    /// # Safety Invariant
    ///
    /// This method is safe to call **within Bevy systems** where `&World`
    /// is guaranteed to outlive the system scope. Using this method outside
    /// of a Bevy system (e.g., in a `main()` function with a locally-owned
    /// `World`) requires that the caller ensure the `World` outlives all
    /// `EntityPtr` instances created from it. Dropping the `World` while
    /// `EntityPtr` instances exist is undefined behavior.
    ///
    /// For fully safe code without this invariant, use
    /// [`EntityHandle::bind()`] and [`BoundEntity`] instead.
    fn entity_ptr(&self, entity: Entity) -> EntityPtr;
}

impl WorldExt for World {
    #[inline]
    fn bind_entity(&self, entity: Entity) -> BoundEntity<'_> {
        EntityHandle::new(entity).bind(self)
    }

    #[inline]
    fn entity_ptr(&self, entity: Entity) -> EntityPtr {
        // SAFETY: Within a Bevy system, &World outlives the system scope.
        // EntityPtr is !Send, preventing escape to other threads.
        unsafe { WorldRef::new(self) }.entity(entity)
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use bevy_ecs::component::Component;
    use bevy_ecs::world::World;

    #[derive(Component)]
    struct Name(&'static str);

    #[derive(Component)]
    struct Health(i32);

    #[derive(Component)]
    struct Parent(EntityHandle);

    #[derive(Component)]
    struct Target(Option<EntityHandle>);

    /// Test mixing EntityHandle and EntityPtr approaches.
    #[test]
    fn mixed_handle_and_ptr() {
        let mut world = World::new();

        // Create a chain: root -> child -> grandchild
        let grandchild = world.spawn(Name("grandchild")).id();
        let child = world
            .spawn((Name("child"), Parent(EntityHandle::new(grandchild))))
            .id();
        let root = world
            .spawn((Name("root"), Parent(EntityHandle::new(child))))
            .id();

        // Use EntityHandle approach
        let handle = EntityHandle::new(root);
        let bound = handle.bind(&world);
        assert_eq!(bound.get::<Name>().unwrap().0, "root");

        // Follow to child using EntityHandle
        let child_bound = bound.follow::<Parent, _>(|p| p.0).unwrap();
        assert_eq!(child_bound.get::<Name>().unwrap().0, "child");

        // Switch to EntityPtr approach
        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.from_handle(child_bound.handle());
        assert_eq!(ptr.get::<Name>().unwrap().0, "child");

        // Follow to grandchild using EntityPtr
        let grandchild_ptr = ptr.follow::<Parent, _>(|p| p.0).unwrap();
        assert_eq!(grandchild_ptr.get::<Name>().unwrap().0, "grandchild");

        // Convert back to handle
        let grandchild_handle = grandchild_ptr.handle();
        assert_eq!(
            grandchild_handle.get::<Name>(&world).unwrap().0,
            "grandchild"
        );
    }

    /// Test tree traversal pattern.
    #[test]
    fn tree_traversal() {
        let mut world = World::new();

        // Build tree:
        //       root
        //      /    \
        //   left   right
        //           |
        //         leaf
        let leaf = world.spawn(Name("leaf")).id();
        let left = world.spawn(Name("left")).id();
        let right = world
            .spawn((Name("right"), Parent(EntityHandle::new(leaf))))
            .id();
        let root = world.spawn(Name("root")).id();

        // Store children via optional targets
        world
            .entity_mut(root)
            .insert(Target(Some(EntityHandle::new(left))));
        world
            .entity_mut(left)
            .insert(Target(Some(EntityHandle::new(right))));

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };

        // Traverse using follow_opt
        let root_ptr = w.entity(root);
        let left_ptr = root_ptr.follow_opt::<Target, _>(|t| t.0).unwrap();
        let right_ptr = left_ptr.follow_opt::<Target, _>(|t| t.0).unwrap();
        let leaf_ptr = right_ptr.follow::<Parent, _>(|p| p.0).unwrap();

        assert_eq!(root_ptr.get::<Name>().unwrap().0, "root");
        assert_eq!(left_ptr.get::<Name>().unwrap().0, "left");
        assert_eq!(right_ptr.get::<Name>().unwrap().0, "right");
        assert_eq!(leaf_ptr.get::<Name>().unwrap().0, "leaf");
    }

    /// Test graceful handling of stale references using EntityHandle.
    ///
    /// Note: EntityPtr cannot be used across mutations - use EntityHandle
    /// when you need to check entity validity after world changes.
    #[test]
    fn stale_reference_handling() {
        let mut world = World::new();

        let target = world.spawn(Name("target")).id();
        let source = world
            .spawn((Name("source"), Parent(EntityHandle::new(target))))
            .id();

        // Use EntityHandle for stale reference handling
        let source_handle = EntityHandle::new(source);

        // Check target exists before despawn
        {
            let bound = source_handle.bind(&world);
            let target_bound = bound.follow::<Parent, _>(|p| p.0).unwrap();
            assert!(target_bound.is_alive());
            assert_eq!(target_bound.get::<Name>().unwrap().0, "target");
        }

        // Despawn target
        world.despawn(target);

        // Rebind after mutation - gracefully handles stale reference
        let bound = source_handle.bind(&world);
        let stale_bound = bound.follow::<Parent, _>(|p| p.0).unwrap();
        assert!(!stale_bound.is_alive());
        assert!(stale_bound.get::<Name>().is_none());
    }

    /// Test EntityHandle can be stored and retrieved from components.
    #[test]
    fn handle_in_component() {
        let mut world = World::new();

        let target = world.spawn((Name("target"), Health(100))).id();
        let source = world
            .spawn((Name("source"), Parent(EntityHandle::new(target))))
            .id();

        // Retrieve handle from component
        let handle = EntityHandle::new(source);
        let parent_handle = handle.bind(&world).get::<Parent>().unwrap().0;

        // Use the retrieved handle
        assert_eq!(parent_handle.get::<Name>(&world).unwrap().0, "target");
        assert_eq!(parent_handle.get::<Health>(&world).unwrap().0, 100);
    }

    // =========================================================================
    // FP Pattern Tests: Recursive Tree Patterns
    // =========================================================================

    #[derive(Component)]
    struct Value(i32);

    #[derive(Component)]
    struct TreeChildren(Vec<EntityHandle>);

    /// Recursive function to sum all values in a tree.
    /// From plan: sum_tree_health example.
    fn sum_tree(ptr: EntityPtr) -> i32 {
        let mine = ptr.get::<Value>().map(|v| v.0).unwrap_or(0);
        let children_sum: i32 = ptr
            .get::<TreeChildren>()
            .map(|c| {
                c.0.iter()
                    .map(|h| sum_tree(EntityPtr::new(h.entity(), ptr.world())))
                    .sum()
            })
            .unwrap_or(0);
        mine + children_sum
    }

    /// Test recursive tree traversal summing values.
    #[test]
    fn sum_tree_values() {
        let mut world = World::new();

        // Build tree:
        //       root (10)
        //      /    \
        //   a (5)   b (3)
        //     |
        //   c (2)
        let c = world.spawn(Value(2)).id();
        let a = world
            .spawn((Value(5), TreeChildren(vec![EntityHandle::new(c)])))
            .id();
        let b = world.spawn(Value(3)).id();
        let root = world
            .spawn((
                Value(10),
                TreeChildren(vec![EntityHandle::new(a), EntityHandle::new(b)]),
            ))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let total = sum_tree(w.entity(root));

        assert_eq!(total, 20); // 10 + 5 + 2 + 3
    }

    /// Recursive function to find the root of a tree by traversing parents.
    fn find_root(ptr: EntityPtr) -> EntityPtr {
        match ptr.follow::<Parent, _>(|p| p.0) {
            Some(parent_ptr) => find_root(parent_ptr),
            None => ptr,
        }
    }

    /// Test recursive parent traversal to find root.
    #[test]
    fn find_root_test() {
        let mut world = World::new();

        // Build chain: root -> a -> b -> c
        let root = world.spawn(Name("root")).id();
        let a = world
            .spawn((Name("a"), Parent(EntityHandle::new(root))))
            .id();
        let b = world.spawn((Name("b"), Parent(EntityHandle::new(a)))).id();
        let c = world.spawn((Name("c"), Parent(EntityHandle::new(b)))).id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };

        // Starting from any node should find root
        assert_eq!(find_root(w.entity(c)).get::<Name>().unwrap().0, "root");
        assert_eq!(find_root(w.entity(b)).get::<Name>().unwrap().0, "root");
        assert_eq!(find_root(w.entity(a)).get::<Name>().unwrap().0, "root");
        assert_eq!(find_root(w.entity(root)).get::<Name>().unwrap().0, "root");
    }

    /// Recursive function to compute tree depth.
    fn tree_depth(ptr: EntityPtr) -> usize {
        ptr.get::<TreeChildren>()
            .map(|c| {
                c.0.iter()
                    .map(|h| tree_depth(EntityPtr::new(h.entity(), ptr.world())))
                    .max()
                    .unwrap_or(0)
                    + 1
            })
            .unwrap_or(0)
    }

    /// Test recursive depth calculation.
    #[test]
    fn tree_depth_test() {
        let mut world = World::new();

        // Build tree with depth 3:
        //       root (depth 3)
        //      /    \
        //   a (2)   b (1)
        //     |
        //   c (1)
        //     |
        //   d (0)
        let d = world.spawn(Name("d")).id();
        let c = world
            .spawn((Name("c"), TreeChildren(vec![EntityHandle::new(d)])))
            .id();
        let a = world
            .spawn((Name("a"), TreeChildren(vec![EntityHandle::new(c)])))
            .id();
        let b = world.spawn(Name("b")).id();
        let root = world
            .spawn((
                Name("root"),
                TreeChildren(vec![EntityHandle::new(a), EntityHandle::new(b)]),
            ))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };

        assert_eq!(tree_depth(w.entity(root)), 3);
        assert_eq!(tree_depth(w.entity(a)), 2);
        assert_eq!(tree_depth(w.entity(c)), 1);
        assert_eq!(tree_depth(w.entity(d)), 0);
    }

    // =========================================================================
    // FP Pattern Tests: Deep Navigation Chains
    // =========================================================================

    /// Test 5+ level entity chain traversal.
    #[test]
    fn deep_chain_navigation() {
        let mut world = World::new();

        // Build 6-level chain: e0 -> e1 -> e2 -> e3 -> e4 -> e5
        let e5 = world.spawn((Name("e5"), Health(5))).id();
        let e4 = world
            .spawn((Name("e4"), Health(4), Parent(EntityHandle::new(e5))))
            .id();
        let e3 = world
            .spawn((Name("e3"), Health(3), Parent(EntityHandle::new(e4))))
            .id();
        let e2 = world
            .spawn((Name("e2"), Health(2), Parent(EntityHandle::new(e3))))
            .id();
        let e1 = world
            .spawn((Name("e1"), Health(1), Parent(EntityHandle::new(e2))))
            .id();
        let e0 = world
            .spawn((Name("e0"), Health(0), Parent(EntityHandle::new(e1))))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.entity(e0);

        // Navigate 5 hops deep
        let deep = ptr
            .follow::<Parent, _>(|p| p.0)
            .and_then(|p| p.follow::<Parent, _>(|p| p.0))
            .and_then(|p| p.follow::<Parent, _>(|p| p.0))
            .and_then(|p| p.follow::<Parent, _>(|p| p.0))
            .and_then(|p| p.follow::<Parent, _>(|p| p.0));

        assert!(deep.is_some());
        assert_eq!(deep.unwrap().get::<Name>().unwrap().0, "e5");
        assert_eq!(deep.unwrap().get::<Health>().unwrap().0, 5);
    }

    /// Test breadth traversal - multiple children at each level.
    #[test]
    fn breadth_traversal() {
        let mut world = World::new();

        // Build wide tree:
        //           root
        //    /    /    \    \
        //   a    b      c    d
        //  /\   /\     /\   /\
        // a0 a1 b0 b1 c0 c1 d0 d1
        let leaves: Vec<_> = ["a0", "a1", "b0", "b1", "c0", "c1", "d0", "d1"]
            .iter()
            .map(|name| world.spawn(Name(name)).id())
            .collect();

        let a = world
            .spawn((
                Name("a"),
                TreeChildren(vec![
                    EntityHandle::new(leaves[0]),
                    EntityHandle::new(leaves[1]),
                ]),
            ))
            .id();
        let b = world
            .spawn((
                Name("b"),
                TreeChildren(vec![
                    EntityHandle::new(leaves[2]),
                    EntityHandle::new(leaves[3]),
                ]),
            ))
            .id();
        let c = world
            .spawn((
                Name("c"),
                TreeChildren(vec![
                    EntityHandle::new(leaves[4]),
                    EntityHandle::new(leaves[5]),
                ]),
            ))
            .id();
        let d = world
            .spawn((
                Name("d"),
                TreeChildren(vec![
                    EntityHandle::new(leaves[6]),
                    EntityHandle::new(leaves[7]),
                ]),
            ))
            .id();

        let root = world
            .spawn((
                Name("root"),
                TreeChildren(vec![
                    EntityHandle::new(a),
                    EntityHandle::new(b),
                    EntityHandle::new(c),
                    EntityHandle::new(d),
                ]),
            ))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let root_ptr = w.entity(root);

        // Collect all leaf names via breadth-like traversal
        let children = root_ptr.get::<TreeChildren>().unwrap();
        let grandchildren: Vec<_> = children
            .0
            .iter()
            .flat_map(|h| {
                let child_ptr = EntityPtr::new(h.entity(), root_ptr.world());
                child_ptr
                    .get::<TreeChildren>()
                    .map(|tc| {
                        tc.0.iter()
                            .map(|gh| EntityPtr::new(gh.entity(), root_ptr.world()))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            })
            .collect();

        assert_eq!(grandchildren.len(), 8);
        let names: Vec<_> = grandchildren
            .iter()
            .filter_map(|p| p.get::<Name>())
            .map(|n| n.0)
            .collect();
        assert!(names.contains(&"a0"));
        assert!(names.contains(&"d1"));
    }

    // =========================================================================
    // FP Pattern Tests: Referential Transparency
    // =========================================================================

    /// Test that same navigation twice returns identical results.
    /// Core FP guarantee: referential transparency.
    #[test]
    fn referential_transparency() {
        let mut world = World::new();

        let target = world.spawn((Name("target"), Health(42))).id();
        let source = world
            .spawn((Name("source"), Parent(EntityHandle::new(target))))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.entity(source);

        // Call the same navigation multiple times
        let result1 = ptr.follow::<Parent, _>(|p| p.0);
        let result2 = ptr.follow::<Parent, _>(|p| p.0);
        let result3 = ptr.follow::<Parent, _>(|p| p.0);

        // All results should be identical
        assert!(result1.is_some());
        assert!(result2.is_some());
        assert!(result3.is_some());

        // Same entity
        assert_eq!(result1.unwrap().entity(), result2.unwrap().entity());
        assert_eq!(result2.unwrap().entity(), result3.unwrap().entity());

        // Same component values
        assert_eq!(
            result1.unwrap().get::<Health>().unwrap().0,
            result2.unwrap().get::<Health>().unwrap().0
        );
    }

    /// Test that chained operations are deterministic (pure function composition).
    #[test]
    fn pure_function_composition() {
        let mut world = World::new();

        // Build chain: a -> b -> c -> d
        let d = world.spawn((Name("d"), Health(4))).id();
        let c = world
            .spawn((Name("c"), Health(3), Parent(EntityHandle::new(d))))
            .id();
        let b = world
            .spawn((Name("b"), Health(2), Parent(EntityHandle::new(c))))
            .id();
        let a = world
            .spawn((Name("a"), Health(1), Parent(EntityHandle::new(b))))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };

        // Define a composed navigation function
        let navigate_three = |ptr: EntityPtr| -> Option<i32> {
            ptr.follow::<Parent, _>(|p| p.0)
                .and_then(|p| p.follow::<Parent, _>(|p| p.0))
                .and_then(|p| p.follow::<Parent, _>(|p| p.0))
                .and_then(|p| p.get::<Health>().map(|h| h.0))
        };

        // Should be deterministic
        let result1 = navigate_three(w.entity(a));
        let result2 = navigate_three(w.entity(a));

        assert_eq!(result1, Some(4));
        assert_eq!(result2, Some(4));
        assert_eq!(result1, result2);
    }

    // =========================================================================
    // FP Pattern Tests: Option Composition Patterns
    // =========================================================================

    /// Test and_then chains through navigation.
    #[test]
    fn option_chain_and_then() {
        let mut world = World::new();

        let end = world.spawn((Name("end"), Health(100))).id();
        let mid = world
            .spawn((Name("mid"), Parent(EntityHandle::new(end))))
            .id();
        let start = world
            .spawn((Name("start"), Parent(EntityHandle::new(mid))))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.entity(start);

        // Chain with and_then
        let health = ptr
            .follow::<Parent, _>(|p| p.0)
            .and_then(|p| p.follow::<Parent, _>(|p| p.0))
            .and_then(|p| p.get::<Health>())
            .map(|h| h.0);

        assert_eq!(health, Some(100));

        // Early termination on None
        let no_health = ptr
            .follow::<Parent, _>(|p| p.0)
            .and_then(|p| p.follow::<Parent, _>(|p| p.0))
            .and_then(|p| p.follow::<Parent, _>(|p| p.0)) // No parent here
            .and_then(|p| p.get::<Health>())
            .map(|h| h.0);

        assert_eq!(no_health, None);
    }

    /// Test filter_map pattern on children.
    #[test]
    fn filter_map_over_children() {
        let mut world = World::new();

        // Some children have Health, some don't
        let healthy1 = world.spawn((Name("healthy1"), Health(50))).id();
        let healthy2 = world.spawn((Name("healthy2"), Health(75))).id();
        let no_health1 = world.spawn(Name("no_health1")).id();
        let no_health2 = world.spawn(Name("no_health2")).id();

        let parent = world
            .spawn((
                Name("parent"),
                TreeChildren(vec![
                    EntityHandle::new(healthy1),
                    EntityHandle::new(no_health1),
                    EntityHandle::new(healthy2),
                    EntityHandle::new(no_health2),
                ]),
            ))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.entity(parent);

        // filter_map pattern: only get children with Health
        let healths: Vec<i32> = ptr
            .get::<TreeChildren>()
            .map(|tc| {
                tc.0.iter()
                    .filter_map(|h| {
                        let child = EntityPtr::new(h.entity(), ptr.world());
                        child.get::<Health>().map(|h| h.0)
                    })
                    .collect()
            })
            .unwrap_or_default();

        assert_eq!(healths.len(), 2);
        assert!(healths.contains(&50));
        assert!(healths.contains(&75));

        // Sum using fold
        let total: i32 = healths.iter().sum();
        assert_eq!(total, 125);
    }

    // =========================================================================
    // FP Pattern Tests: Multi-Component Navigation
    // =========================================================================

    #[derive(Component)]
    struct TypeA(EntityHandle);

    #[derive(Component)]
    struct TypeB(EntityHandle);

    /// Test navigation through different component types at each hop.
    #[test]
    fn multi_component_chain() {
        let mut world = World::new();

        // Chain: start -[TypeA]-> mid -[TypeB]-> end
        let end = world.spawn((Name("end"), Health(99))).id();
        let mid = world
            .spawn((Name("mid"), TypeB(EntityHandle::new(end))))
            .id();
        let start = world
            .spawn((Name("start"), TypeA(EntityHandle::new(mid))))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.entity(start);

        // Navigate through different component types
        let result = ptr
            .follow::<TypeA, _>(|a| a.0)
            .and_then(|p| p.follow::<TypeB, _>(|b| b.0))
            .and_then(|p| p.get::<Health>())
            .map(|h| h.0);

        assert_eq!(result, Some(99));
    }

    /// Test alternating component navigation: A -> B -> A -> B pattern.
    #[test]
    fn alternating_component_navigation() {
        let mut world = World::new();

        // Build alternating chain: e0 -A-> e1 -B-> e2 -A-> e3 -B-> e4
        let e4 = world.spawn((Name("e4"), Health(44))).id();
        let e3 = world.spawn((Name("e3"), TypeB(EntityHandle::new(e4)))).id();
        let e2 = world.spawn((Name("e2"), TypeA(EntityHandle::new(e3)))).id();
        let e1 = world.spawn((Name("e1"), TypeB(EntityHandle::new(e2)))).id();
        let e0 = world.spawn((Name("e0"), TypeA(EntityHandle::new(e1)))).id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let ptr = w.entity(e0);

        // Navigate A -> B -> A -> B
        let result = ptr
            .follow::<TypeA, _>(|a| a.0)
            .and_then(|p| p.follow::<TypeB, _>(|b| b.0))
            .and_then(|p| p.follow::<TypeA, _>(|a| a.0))
            .and_then(|p| p.follow::<TypeB, _>(|b| b.0));

        assert!(result.is_some());
        assert_eq!(result.unwrap().get::<Name>().unwrap().0, "e4");
        assert_eq!(result.unwrap().get::<Health>().unwrap().0, 44);
    }

    // =========================================================================
    // WorldExt Extension Trait Tests
    // =========================================================================

    /// Test WorldExt::entity_ptr - no unsafe needed!
    #[test]
    fn world_ext_entity_ptr() {
        let mut world = World::new();
        let entity = world.spawn(Name("test")).id();

        // Extension trait usage - no unsafe!
        let ptr = world.entity_ptr(entity);
        assert_eq!(ptr.get::<Name>().unwrap().0, "test");
    }

    /// Test WorldExt::bind_entity - convenience method.
    #[test]
    fn world_ext_bind_entity() {
        let mut world = World::new();
        let entity = world.spawn(Name("test")).id();

        let bound = world.bind_entity(entity);
        assert_eq!(bound.get::<Name>().unwrap().0, "test");
    }

    /// Test EntityPtr Eq and Hash implementations.
    #[test]
    fn entity_ptr_eq_hash() {
        let mut world = World::new();
        let e1 = world.spawn(()).id();
        let e2 = world.spawn(()).id();

        let ptr1 = world.entity_ptr(e1);
        let ptr1_copy = world.entity_ptr(e1);
        let ptr2 = world.entity_ptr(e2);

        // Test equality
        assert_eq!(ptr1, ptr1_copy);
        assert_ne!(ptr1, ptr2);

        // Test works in HashSet
        let mut set = std::collections::HashSet::new();
        set.insert(ptr1);
        assert!(set.contains(&ptr1_copy));
        assert!(!set.contains(&ptr2));
    }
}

#[cfg(all(test, feature = "nav-traits"))]
mod nav_integration_tests {
    use super::*;
    use bevy_ecs::component::Component;
    use bevy_ecs::world::World;

    #[derive(Component)]
    struct Name(&'static str);

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

    /// Test parent navigation with BoundEntity.
    #[test]
    fn bound_parent_chain() {
        let mut world = World::new();

        let grandparent = world.spawn(Name("grandparent")).id();
        let parent = world
            .spawn((
                Name("parent"),
                ParentRef(Some(EntityHandle::new(grandparent))),
            ))
            .id();
        let child = world
            .spawn((Name("child"), ParentRef(Some(EntityHandle::new(parent)))))
            .id();

        let bound = EntityHandle::new(child).bind(&world);

        // Navigate up the chain
        let parent_bound = bound.nav().parent::<ParentRef>().unwrap();
        assert_eq!(parent_bound.get::<Name>().unwrap().0, "parent");

        let grandparent_bound = parent_bound.nav().parent::<ParentRef>().unwrap();
        assert_eq!(grandparent_bound.get::<Name>().unwrap().0, "grandparent");
    }

    /// Test children navigation with EntityPtr.
    #[test]
    fn ptr_children_iteration() {
        let mut world = World::new();

        let child1 = world.spawn(Name("child1")).id();
        let child2 = world.spawn(Name("child2")).id();
        let child3 = world.spawn(Name("child3")).id();
        let parent = world
            .spawn((
                Name("parent"),
                ChildRefs(vec![
                    EntityHandle::new(child1),
                    EntityHandle::new(child2),
                    EntityHandle::new(child3),
                ]),
            ))
            .id();

        // SAFETY: world outlives usage
        let w = unsafe { WorldRef::new(&world) };
        let parent_ptr = w.entity(parent);

        let children: Vec<_> = parent_ptr.nav_many().children::<ChildRefs>().collect();
        assert_eq!(children.len(), 3);

        let names: Vec<_> = children
            .iter()
            .filter_map(|c| c.get::<Name>())
            .map(|n| n.0)
            .collect();
        assert!(names.contains(&"child1"));
        assert!(names.contains(&"child2"));
        assert!(names.contains(&"child3"));
    }
}
