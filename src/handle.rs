//! EntityHandle and BoundEntity - fully safe foundation for entity access.
//!
//! This module provides safe, explicit entity access requiring a `&World` parameter.

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;

/// A lightweight handle to an entity that can be stored in components.
///
/// This is a newtype over Bevy's `Entity` that provides ergonomic access methods
/// while requiring explicit `&World` parameters. Safe to store and share across threads.
///
/// # Size
/// 8 bytes (same as `Entity`)
///
/// # Thread Safety
/// `Send + Sync` - safe to store in components and share between threads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct EntityHandle(Entity);

impl EntityHandle {
    /// Creates a new handle from an entity.
    #[inline]
    pub const fn new(entity: Entity) -> Self {
        Self(entity)
    }

    /// Returns the underlying `Entity`.
    #[inline]
    pub const fn entity(self) -> Entity {
        self.0
    }

    /// Gets a component from the referenced entity.
    ///
    /// Returns `None` if the entity doesn't exist or doesn't have the component.
    #[inline]
    #[must_use]
    pub fn get<T: Component>(self, world: &World) -> Option<&T> {
        world.get::<T>(self.0)
    }

    /// Checks if the entity has a component of type `T`.
    ///
    /// Returns `false` if the entity doesn't exist.
    #[inline]
    pub fn has<T: Component>(self, world: &World) -> bool {
        world.get::<T>(self.0).is_some()
    }

    /// Checks if the referenced entity is still alive.
    #[inline]
    pub fn is_alive(self, world: &World) -> bool {
        world.get_entity(self.0).is_ok()
    }

    /// Binds this handle to a world, creating a `BoundEntity` for fluent access.
    #[inline]
    pub fn bind(self, world: &World) -> BoundEntity<'_> {
        BoundEntity::new(self.0, world)
    }
}

impl std::fmt::Display for EntityHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntityHandle({})", self.0)
    }
}

impl From<Entity> for EntityHandle {
    #[inline]
    fn from(entity: Entity) -> Self {
        Self::new(entity)
    }
}

impl From<EntityHandle> for Entity {
    #[inline]
    fn from(handle: EntityHandle) -> Self {
        handle.0
    }
}

// Send + Sync auto-derived: EntityHandle is #[repr(transparent)] over Entity,
// which is Send + Sync.

/// An entity bound to a world reference for fluent, scoped access.
///
/// Created by calling `EntityHandle::bind()` or directly. Provides method chaining
/// without repeatedly passing `&World`.
///
/// # Size
/// 16 bytes (`Entity` + `&World`)
///
/// # Thread Safety
/// NOT `Send` - borrows `&World` so must stay on the creating thread.
#[derive(Clone, Copy)]
pub struct BoundEntity<'w> {
    entity: Entity,
    world: &'w World,
}

impl<'w> BoundEntity<'w> {
    /// Creates a new bound entity.
    #[inline]
    pub const fn new(entity: Entity, world: &'w World) -> Self {
        Self { entity, world }
    }

    /// Returns the underlying `Entity`.
    #[inline]
    pub const fn entity(self) -> Entity {
        self.entity
    }

    /// Returns an `EntityHandle` for storage.
    #[inline]
    pub const fn handle(self) -> EntityHandle {
        EntityHandle(self.entity)
    }

    /// Gets a component from this entity.
    ///
    /// Returns `None` if the entity doesn't exist or doesn't have the component.
    #[inline]
    #[must_use]
    pub fn get<T: Component>(self) -> Option<&'w T> {
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
    pub fn follow<T, F>(self, f: F) -> Option<BoundEntity<'w>>
    where
        T: Component,
        F: FnOnce(&T) -> EntityHandle,
    {
        self.get::<T>().map(|c| f(c).bind(self.world))
    }

    /// Follows an optional reference component to another entity.
    ///
    /// The extractor function returns `Option<EntityHandle>`.
    ///
    /// Returns `None` if this entity doesn't have the component or the reference is None.
    #[inline]
    #[must_use]
    pub fn follow_opt<T, F>(self, f: F) -> Option<BoundEntity<'w>>
    where
        T: Component,
        F: FnOnce(&T) -> Option<EntityHandle>,
    {
        self.get::<T>()
            .and_then(|c| f(c).map(|h| h.bind(self.world)))
    }

    /// Returns a navigator for this entity, enabling `HasParent`/`HasChildren` navigation.
    ///
    /// This method is always available but navigation methods require the `nav-traits` feature.
    #[inline]
    pub const fn nav(self) -> BoundEntityNav<'w> {
        BoundEntityNav(self)
    }

    /// Returns the world reference.
    ///
    /// This allows access to the underlying world for advanced use cases.
    #[inline]
    pub const fn world(self) -> &'w World {
        self.world
    }
}

impl std::fmt::Debug for BoundEntity<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoundEntity")
            .field("entity", &self.entity)
            .finish_non_exhaustive()
    }
}

/// Navigation wrapper for `BoundEntity`, providing parent/children traversal.
///
/// Created by calling `BoundEntity::nav()`. Methods are gated behind `nav-traits` feature.
#[derive(Clone, Copy)]
pub struct BoundEntityNav<'w>(pub(crate) BoundEntity<'w>);

impl<'w> BoundEntityNav<'w> {
    /// Returns the underlying `BoundEntity`.
    #[inline]
    pub const fn inner(self) -> BoundEntity<'w> {
        self.0
    }
}

impl std::fmt::Debug for BoundEntityNav<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BoundEntityNav").field(&self.0).finish()
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
    fn entity_handle_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EntityHandle>();
    }

    #[test]
    fn entity_handle_display() {
        let entity = Entity::from_raw_u32(42).unwrap();
        let handle = EntityHandle::new(entity);
        let display = format!("{}", handle);
        assert!(display.starts_with("EntityHandle("));
    }

    #[test]
    fn handle_roundtrip() {
        let entity = Entity::from_raw_u32(42).unwrap();
        let handle = EntityHandle::new(entity);
        assert_eq!(handle.entity(), entity);

        // From conversions
        let handle2: EntityHandle = entity.into();
        let entity2: Entity = handle2.into();
        assert_eq!(entity, entity2);
    }

    #[test]
    fn handle_component_access() {
        let mut world = World::new();
        let entity = world.spawn((Name("test"), Health(100))).id();
        let handle = EntityHandle::new(entity);

        assert_eq!(handle.get::<Name>(&world).unwrap().0, "test");
        assert_eq!(handle.get::<Health>(&world).unwrap().0, 100);
        assert!(handle.has::<Name>(&world));
        assert!(handle.has::<Health>(&world));
        assert!(!handle.has::<Target>(&world));
    }

    #[test]
    fn handle_stale_entity() {
        let mut world = World::new();
        let entity = world.spawn(Name("temporary")).id();
        let handle = EntityHandle::new(entity);

        assert!(handle.is_alive(&world));
        assert!(handle.get::<Name>(&world).is_some());

        world.despawn(entity);

        // Graceful None, not UB
        assert!(!handle.is_alive(&world));
        assert!(handle.get::<Name>(&world).is_none());
    }

    #[test]
    fn bound_entity_access() {
        let mut world = World::new();
        let entity = world.spawn((Name("bound"), Health(50))).id();
        let handle = EntityHandle::new(entity);

        let bound = handle.bind(&world);
        assert_eq!(bound.entity(), entity);
        assert_eq!(bound.handle().entity(), entity);
        assert_eq!(bound.get::<Name>().unwrap().0, "bound");
        assert!(bound.has::<Health>());
        assert!(bound.is_alive());
    }

    #[test]
    fn bound_entity_follow() {
        let mut world = World::new();
        let target_entity = world.spawn(Name("target")).id();
        let source_entity = world
            .spawn((Name("source"), Target(EntityHandle::new(target_entity))))
            .id();

        let source = EntityHandle::new(source_entity).bind(&world);
        let target = source.follow::<Target, _>(|t| t.0).unwrap();

        assert_eq!(target.get::<Name>().unwrap().0, "target");
    }

    #[test]
    fn bound_entity_follow_opt() {
        let mut world = World::new();
        let target_entity = world.spawn(Name("target")).id();

        // Entity with Some target
        let with_target = world
            .spawn(OptionalTarget(Some(EntityHandle::new(target_entity))))
            .id();

        // Entity with None target
        let without_target = world.spawn(OptionalTarget(None)).id();

        let with = EntityHandle::new(with_target).bind(&world);
        let without = EntityHandle::new(without_target).bind(&world);

        assert!(with.follow_opt::<OptionalTarget, _>(|t| t.0).is_some());
        assert!(without.follow_opt::<OptionalTarget, _>(|t| t.0).is_none());
    }

    #[test]
    fn bound_entity_stale() {
        let mut world = World::new();
        let entity = world.spawn(Name("temp")).id();
        let handle = EntityHandle::new(entity);
        let bound = handle.bind(&world);

        assert!(bound.is_alive());

        world.despawn(entity);

        // Re-bind after despawn
        let bound2 = handle.bind(&world);
        assert!(!bound2.is_alive());
        assert!(bound2.get::<Name>().is_none());
    }

    #[test]
    fn memory_layout() {
        assert_eq!(std::mem::size_of::<EntityHandle>(), 8);
        assert_eq!(std::mem::size_of::<BoundEntity<'_>>(), 16);
    }

    #[test]
    fn nav_wrapper_access() {
        let mut world = World::new();
        let entity = world.spawn(Name("nav")).id();
        let bound = EntityHandle::new(entity).bind(&world);
        let nav = bound.nav();

        assert_eq!(nav.inner().entity(), entity);
    }
}
