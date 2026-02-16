//! WorldRef and EntityPtr - ergonomic 'static approach with single unsafe boundary.
//!
//! This module provides an ergonomic API that avoids repeatedly passing `&World` by
//! transmuting the lifetime to `'static`. The single unsafe point is `WorldRef::new()`.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

use crate::handle::EntityHandle;

/// A reference to a World with erased lifetime for ergonomic entity traversal.
///
/// This is the single unsafe boundary in the crate. All `EntityPtr` instances
/// created from a `WorldRef` will borrow from the original World.
///
/// # Size
/// 8 bytes (`&'static World`)
///
/// # Safety
/// The caller must ensure:
/// 1. The World outlives all `EntityPtr` instances created from this `WorldRef`
/// 2. **The World is NOT mutated** while any `EntityPtr` exists
///
/// In Bevy systems, this is naturally satisfied: systems with `&World` access
/// cannot mutate. Create `WorldRef` at system entry, use it for reads, and let
/// it drop before the system returns.
///
/// For stale reference handling across mutations, use `EntityHandle` instead.
///
/// # Thread Safety
/// NOT `Send`, NOT `Sync` - must stay on the creating thread within a single system.
#[derive(Clone, Copy)]
pub struct WorldRef {
    world: &'static World,
}

impl WorldRef {
    /// Creates a new WorldRef by transmuting the lifetime to 'static.
    ///
    /// # Safety
    /// The caller must ensure that:
    /// - The World reference outlives all `EntityPtr` instances created from this `WorldRef`
    /// - The World is NOT mutated while any `EntityPtr` from this `WorldRef` exists
    /// - Typically this means the `WorldRef` is created at system entry and discarded at system exit
    #[inline]
    pub unsafe fn new(world: &World) -> Self {
        Self {
            // SAFETY: Caller guarantees the World outlives all EntityPtrs
            // and is not mutated while any EntityPtr exists.
            world: unsafe { std::mem::transmute::<&World, &'static World>(world) },
        }
    }

    /// Gets an EntityPtr for the given entity.
    ///
    /// Returns an `EntityPtr` regardless of whether the entity exists.
    /// Use `EntityPtr::is_alive()` to check validity.
    #[inline]
    pub fn entity(&self, entity: Entity) -> EntityPtr {
        EntityPtr::new(entity, self.world)
    }

    /// Gets an EntityPtr only if the entity exists.
    ///
    /// Returns `None` if the entity has been despawned.
    #[inline]
    #[must_use]
    pub fn entity_opt(&self, entity: Entity) -> Option<EntityPtr> {
        if self.world.get_entity(entity).is_ok() {
            Some(EntityPtr::new(entity, self.world))
        } else {
            None
        }
    }

    /// Creates an EntityPtr from an EntityHandle.
    #[inline]
    pub fn from_handle(&self, handle: EntityHandle) -> EntityPtr {
        EntityPtr::new(handle.entity(), self.world)
    }

    /// Gets a component directly from an entity.
    ///
    /// Convenience method for one-off component access without creating an EntityPtr.
    #[inline]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.world.get::<T>(entity)
    }

    /// Returns the underlying World reference.
    ///
    /// This can be used to access World methods directly when needed.
    #[inline]
    pub fn world(&self) -> &World {
        self.world
    }
}

impl std::fmt::Debug for WorldRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorldRef").finish_non_exhaustive()
    }
}

// Explicitly NOT implementing Send or Sync - WorldRef must stay on creating thread

/// An ergonomic smart pointer to an entity with embedded World reference.
///
/// Created from `WorldRef::entity()`. Provides fluent method chaining without
/// repeatedly passing `&World`.
///
/// # Size
/// 16 bytes (`Entity` + `&'static World`)
///
/// # Thread Safety
/// NOT `Send`, NOT `Sync` - must stay on the creating thread.
#[derive(Clone, Copy)]
pub struct EntityPtr {
    entity: Entity,
    world: &'static World,
}

impl EntityPtr {
    /// Creates a new EntityPtr (internal use - prefer `WorldRef::entity()`).
    #[inline]
    pub(crate) const fn new(entity: Entity, world: &'static World) -> Self {
        Self { entity, world }
    }

    /// Returns the underlying `Entity`.
    #[inline]
    pub const fn entity(self) -> Entity {
        self.entity
    }

    /// Returns an `EntityHandle` for storage in components.
    #[inline]
    pub const fn handle(self) -> EntityHandle {
        EntityHandle::new(self.entity)
    }

    /// Gets a component from this entity.
    ///
    /// Returns `None` if the entity doesn't exist or doesn't have the component.
    ///
    /// # Lifetime Note
    /// The returned reference has a `'static` lifetime because `EntityPtr` carries
    /// a `'static` world reference. This is safe as long as the `WorldRef` safety
    /// contract is upheld (world outlives all `EntityPtr`s). However, this means
    /// the borrow checker cannot prevent use-after-free if the world is dropped
    /// while references are held. Always ensure `EntityPtr` usage is scoped within
    /// a single system execution.
    #[inline]
    #[must_use]
    pub fn get<T: Component>(self) -> Option<&'static T> {
        self.world.get::<T>(self.entity)
    }

    /// Checks if this entity has a component of type `T`.
    ///
    /// Returns `false` if the entity doesn't exist.
    #[inline]
    pub fn has<T: Component>(self) -> bool {
        self.world.get::<T>(self.entity).is_some()
    }

    /// Checks if this entity is still alive.
    #[inline]
    pub fn is_alive(self) -> bool {
        self.world.get_entity(self.entity).is_ok()
    }

    /// Follows a reference component to another entity.
    ///
    /// The component must contain an `EntityHandle`. Use `follow_opt` for optional references.
    ///
    /// Returns `None` if this entity doesn't have the component.
    #[inline]
    #[must_use]
    pub fn follow<T, F>(self, f: F) -> Option<EntityPtr>
    where
        T: Component,
        F: FnOnce(&T) -> EntityHandle,
    {
        self.get::<T>()
            .map(|c| EntityPtr::new(f(c).entity(), self.world))
    }

    /// Follows an optional reference component to another entity.
    ///
    /// The extractor function returns `Option<EntityHandle>`.
    ///
    /// Returns `None` if this entity doesn't have the component or the reference is None.
    #[inline]
    #[must_use]
    pub fn follow_opt<T, F>(self, f: F) -> Option<EntityPtr>
    where
        T: Component,
        F: FnOnce(&T) -> Option<EntityHandle>,
    {
        self.get::<T>()
            .and_then(|c| f(c).map(|h| EntityPtr::new(h.entity(), self.world)))
    }

    /// Creates an EntityPtr from an EntityHandle using this pointer's world.
    ///
    /// Convenience method for tree traversal when you have stored handles.
    /// This enables cleaner recursive patterns without needing access to `WorldRef`.
    ///
    /// # Example
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_entity_ptr::{EntityHandle, EntityPtr};
    ///
    /// #[derive(Component)]
    /// struct TreeChildren(Vec<EntityHandle>);
    ///
    /// #[derive(Component)]
    /// struct Value(i32);
    ///
    /// fn sum_tree(ptr: EntityPtr) -> i32 {
    ///     let children_sum: i32 = ptr
    ///         .get::<TreeChildren>()
    ///         .map(|c| c.0.iter().map(|h| sum_tree(ptr.follow_handle(*h))).sum())
    ///         .unwrap_or(0);
    ///     ptr.get::<Value>().map(|v| v.0).unwrap_or(0) + children_sum
    /// }
    /// ```
    #[inline]
    pub fn follow_handle(self, handle: EntityHandle) -> EntityPtr {
        EntityPtr::new(handle.entity(), self.world)
    }

    /// Returns a navigator for this entity, enabling `HasParent`/`HasChildren` navigation.
    ///
    /// This method is always available but navigation methods require the `nav-traits` feature.
    #[inline]
    pub const fn nav(self) -> EntityPtrNav {
        EntityPtrNav(self)
    }

    /// Returns an iterator-like navigator for collecting multiple related entities.
    ///
    /// Useful for iterating over children or other entity collections.
    #[inline]
    pub const fn nav_many(self) -> EntityPtrNavMany {
        EntityPtrNavMany(self)
    }

    /// Returns the world reference (used by nav module).
    #[inline]
    #[cfg_attr(not(feature = "nav-traits"), allow(dead_code))]
    pub(crate) const fn world(self) -> &'static World {
        self.world
    }
}

impl std::fmt::Debug for EntityPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityPtr")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

impl PartialEq for EntityPtr {
    /// Compares by entity ID only.
    ///
    /// This assumes both `EntityPtr`s reference the same world, which is
    /// the typical usage pattern within a single system.
    fn eq(&self, other: &Self) -> bool {
        self.entity == other.entity
    }
}

impl Eq for EntityPtr {}

impl std::hash::Hash for EntityPtr {
    /// Hashes the entity ID only.
    ///
    /// This enables use in `HashSet` and as `HashMap` keys within
    /// a single-world context (the typical usage pattern).
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.entity.hash(state);
    }
}

// Explicitly NOT implementing Send or Sync - EntityPtr must stay on creating thread

/// Navigation wrapper for `EntityPtr`, providing parent/children traversal.
///
/// Created by calling `EntityPtr::nav()`. Methods are gated behind `nav-traits` feature.
#[derive(Clone, Copy)]
pub struct EntityPtrNav(pub(crate) EntityPtr);

impl EntityPtrNav {
    /// Returns the underlying `EntityPtr`.
    #[inline]
    pub const fn inner(self) -> EntityPtr {
        self.0
    }
}

impl std::fmt::Debug for EntityPtrNav {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EntityPtrNav").field(&self.0).finish()
    }
}

/// Many-navigation wrapper for `EntityPtr`, for collecting multiple related entities.
///
/// Created by calling `EntityPtr::nav_many()`. Methods are gated behind `nav-traits` feature.
#[derive(Clone, Copy)]
pub struct EntityPtrNavMany(pub(crate) EntityPtr);

impl EntityPtrNavMany {
    /// Returns the underlying `EntityPtr`.
    #[inline]
    pub const fn inner(self) -> EntityPtr {
        self.0
    }
}

impl std::fmt::Debug for EntityPtrNavMany {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("EntityPtrNavMany").field(&self.0).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Component)]
    struct Name(&'static str);

    #[derive(Component)]
    struct Health(i32);

    #[derive(Component)]
    struct Target(EntityHandle);

    #[derive(Component)]
    struct OptionalTarget(Option<EntityHandle>);

    #[test]
    fn worldref_is_copy() {
        let mut world = World::new();
        let _entity = world.spawn(Name("test")).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let w = unsafe { WorldRef::new(&world) };
        let w2 = w; // Copy
        let w3 = w; // Still usable after copy
        assert_eq!(std::mem::size_of_val(&w2), std::mem::size_of_val(&w3));
    }

    #[test]
    fn worldref_entity_access() {
        let mut world = World::new();
        let entity = world.spawn((Name("test"), Health(100))).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let ptr = world_ref.entity(entity);
        assert_eq!(ptr.get::<Name>().unwrap().0, "test");
        assert_eq!(ptr.get::<Health>().unwrap().0, 100);
        assert!(ptr.is_alive());
    }

    #[test]
    fn worldref_entity_opt() {
        let mut world = World::new();
        let entity = world.spawn(Name("exists")).id();
        let fake = Entity::from_raw_u32(9999).unwrap();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        assert!(world_ref.entity_opt(entity).is_some());
        assert!(world_ref.entity_opt(fake).is_none());
    }

    #[test]
    fn worldref_from_handle() {
        let mut world = World::new();
        let entity = world.spawn(Name("handle")).id();
        let handle = EntityHandle::new(entity);

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let ptr = world_ref.from_handle(handle);
        assert_eq!(ptr.get::<Name>().unwrap().0, "handle");
    }

    #[test]
    fn worldref_get_direct() {
        let mut world = World::new();
        let entity = world.spawn(Name("direct")).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        assert_eq!(world_ref.get::<Name>(entity).unwrap().0, "direct");
    }

    #[test]
    fn entityptr_handle_conversion() {
        let mut world = World::new();
        let entity = world.spawn(Name("convert")).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let ptr = world_ref.entity(entity);
        assert_eq!(ptr.entity(), entity);
        assert_eq!(ptr.handle().entity(), entity);
    }

    #[test]
    fn entityptr_follow() {
        let mut world = World::new();
        let target_entity = world.spawn(Name("target")).id();
        let source_entity = world
            .spawn((Name("source"), Target(EntityHandle::new(target_entity))))
            .id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let source = world_ref.entity(source_entity);
        let target = source.follow::<Target, _>(|t| t.0).unwrap();

        assert_eq!(target.get::<Name>().unwrap().0, "target");
    }

    #[test]
    fn entityptr_follow_opt() {
        let mut world = World::new();
        let target_entity = world.spawn(Name("target")).id();

        let with_target = world
            .spawn(OptionalTarget(Some(EntityHandle::new(target_entity))))
            .id();
        let without_target = world.spawn(OptionalTarget(None)).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let with = world_ref.entity(with_target);
        let without = world_ref.entity(without_target);

        assert!(with.follow_opt::<OptionalTarget, _>(|t| t.0).is_some());
        assert!(without.follow_opt::<OptionalTarget, _>(|t| t.0).is_none());
    }

    #[test]
    fn entityptr_follow_handle() {
        let mut world = World::new();
        let target_entity = world.spawn(Name("target")).id();
        let source_entity = world.spawn(Name("source")).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let source = world_ref.entity(source_entity);
        let handle = EntityHandle::new(target_entity);

        // follow_handle lets us convert a handle to an EntityPtr using source's world
        let target = source.follow_handle(handle);

        assert_eq!(target.entity(), target_entity);
        assert_eq!(target.get::<Name>().unwrap().0, "target");
    }

    // Note: Stale reference tests for EntityPtr are NOT valid because
    // mutating the World while holding EntityPtrs is undefined behavior.
    // Use EntityHandle for stale reference handling across mutations.
    // See handle::tests::handle_stale_entity and integration_tests::stale_reference_handling.

    #[test]
    fn memory_layout() {
        assert_eq!(std::mem::size_of::<WorldRef>(), 8);
        assert_eq!(std::mem::size_of::<EntityPtr>(), 16);
    }

    #[test]
    fn nav_wrapper_access() {
        let mut world = World::new();
        let entity = world.spawn(Name("nav")).id();

        // SAFETY: world outlives the WorldRef usage in this test
        let world_ref = unsafe { WorldRef::new(&world) };

        let ptr = world_ref.entity(entity);
        let nav = ptr.nav();
        let nav_many = ptr.nav_many();

        assert_eq!(nav.inner().entity(), entity);
        assert_eq!(nav_many.inner().entity(), entity);
    }
}
