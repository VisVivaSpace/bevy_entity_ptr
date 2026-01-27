//! Navigation traits for parent/children traversal.
//!
//! This module provides traits for components that define entity relationships.
//! Feature-gated behind `nav-traits`.

use bevy_ecs::entity::Entity;

/// Trait for components that reference a parent entity.
///
/// Implement this on your component type to enable `.nav().parent()` navigation.
///
/// # Example
/// ```ignore
/// #[derive(Component)]
/// struct Parent(Entity);
///
/// impl HasParent for Parent {
///     fn parent_entity(&self) -> Option<Entity> {
///         Some(self.0)
///     }
/// }
/// ```
pub trait HasParent {
    /// Returns the parent entity, if one exists.
    fn parent_entity(&self) -> Option<Entity>;
}

/// Trait for components that reference child entities.
///
/// Implement this on your component type to enable `.nav_many().children()` navigation.
///
/// # Example
/// ```ignore
/// #[derive(Component)]
/// struct Children(Vec<Entity>);
///
/// impl HasChildren for Children {
///     fn children_entities(&self) -> &[Entity] {
///         &self.0
///     }
/// }
/// ```
pub trait HasChildren {
    /// Returns a slice of child entities.
    fn children_entities(&self) -> &[Entity];
}

// Extension implementations for BoundEntity navigation

use crate::handle::{BoundEntity, BoundEntityNav};

impl<'w> BoundEntityNav<'w> {
    /// Navigates to the parent entity using a component that implements `HasParent`.
    ///
    /// Returns `None` if this entity doesn't have the component or has no parent.
    #[inline]
    pub fn parent<T: bevy_ecs::component::Component + HasParent>(self) -> Option<BoundEntity<'w>> {
        self.0.get::<T>().and_then(|c| {
            c.parent_entity()
                .map(|e| BoundEntity::new(e, self.0.world()))
        })
    }

    /// Navigates to child entities using a component that implements `HasChildren`.
    ///
    /// Returns a Vec of `BoundEntity` for each child. Returns empty Vec if the component is missing.
    #[inline]
    pub fn children<T: bevy_ecs::component::Component + HasChildren>(self) -> Vec<BoundEntity<'w>> {
        self.0
            .get::<T>()
            .map(|c| {
                c.children_entities()
                    .iter()
                    .map(|&e| BoundEntity::new(e, self.0.world()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

// Extension implementations for EntityPtr navigation

use crate::ptr::{EntityPtr, EntityPtrNav, EntityPtrNavMany};

impl EntityPtrNav {
    /// Navigates to the parent entity using a component that implements `HasParent`.
    ///
    /// Returns `None` if this entity doesn't have the component or has no parent.
    #[inline]
    pub fn parent<T: bevy_ecs::component::Component + HasParent>(self) -> Option<EntityPtr> {
        self.0
            .get::<T>()
            .and_then(|c| c.parent_entity().map(|e| EntityPtr::new(e, self.0.world())))
    }
}

impl EntityPtrNavMany {
    /// Navigates to child entities using a component that implements `HasChildren`.
    ///
    /// Returns a Vec of `EntityPtr` for each child. Returns empty Vec if the component is missing.
    #[inline]
    pub fn children<T: bevy_ecs::component::Component + HasChildren>(self) -> Vec<EntityPtr> {
        self.0
            .get::<T>()
            .map(|c| {
                c.children_entities()
                    .iter()
                    .map(|&e| EntityPtr::new(e, self.0.world()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::EntityHandle;
    use crate::ptr::WorldRef;
    use bevy_ecs::component::Component;
    use bevy_ecs::world::World;

    #[derive(Component)]
    struct Name(&'static str);

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

    #[test]
    fn bound_entity_nav_parent() {
        let mut world = World::new();
        let parent = world.spawn(Name("parent")).id();
        let child = world.spawn((Name("child"), ParentRef(Some(parent)))).id();

        let bound = EntityHandle::new(child).bind(&world);
        let parent_bound = bound.nav().parent::<ParentRef>().unwrap();

        assert_eq!(parent_bound.get::<Name>().unwrap().0, "parent");
    }

    #[test]
    fn bound_entity_nav_parent_none() {
        let mut world = World::new();
        let orphan = world.spawn((Name("orphan"), ParentRef(None))).id();

        let bound = EntityHandle::new(orphan).bind(&world);
        assert!(bound.nav().parent::<ParentRef>().is_none());
    }

    #[test]
    fn bound_entity_nav_children() {
        let mut world = World::new();
        let child1 = world.spawn(Name("child1")).id();
        let child2 = world.spawn(Name("child2")).id();
        let parent = world
            .spawn((Name("parent"), ChildRefs(vec![child1, child2])))
            .id();

        let bound = EntityHandle::new(parent).bind(&world);
        let children = bound.nav().children::<ChildRefs>();

        assert_eq!(children.len(), 2);
        let names: Vec<_> = children
            .iter()
            .map(|c| c.get::<Name>().unwrap().0)
            .collect();
        assert!(names.contains(&"child1"));
        assert!(names.contains(&"child2"));
    }

    #[test]
    fn entityptr_nav_parent() {
        let mut world = World::new();
        let parent = world.spawn(Name("parent")).id();
        let child = world.spawn((Name("child"), ParentRef(Some(parent)))).id();

        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(child);
        let parent_ptr = ptr.nav().parent::<ParentRef>().unwrap();

        assert_eq!(parent_ptr.get::<Name>().unwrap().0, "parent");
    }

    #[test]
    fn entityptr_nav_many_children() {
        let mut world = World::new();
        let child1 = world.spawn(Name("child1")).id();
        let child2 = world.spawn(Name("child2")).id();
        let parent = world
            .spawn((Name("parent"), ChildRefs(vec![child1, child2])))
            .id();

        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(parent);
        let children = ptr.nav_many().children::<ChildRefs>();

        assert_eq!(children.len(), 2);
        let names: Vec<_> = children
            .iter()
            .map(|c| c.get::<Name>().unwrap().0)
            .collect();
        assert!(names.contains(&"child1"));
        assert!(names.contains(&"child2"));
    }

    // =========================================================================
    // Empty Collection Edge Cases
    // =========================================================================

    /// Test HasChildren with empty Vec returns empty result.
    #[test]
    fn children_empty_vec() {
        let mut world = World::new();
        let parent = world.spawn((Name("parent"), ChildRefs(vec![]))).id();

        // Test with BoundEntity
        let bound = EntityHandle::new(parent).bind(&world);
        let children = bound.nav().children::<ChildRefs>();
        assert!(children.is_empty());

        // Test with EntityPtr
        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(parent);
        let children = ptr.nav_many().children::<ChildRefs>();
        assert!(children.is_empty());
    }

    /// Test nav_many().children() returns empty Vec when component is missing.
    #[test]
    fn nav_many_no_children_component() {
        let mut world = World::new();
        let entity = world.spawn(Name("no_children")).id();

        // Test with BoundEntity - no ChildRefs component
        let bound = EntityHandle::new(entity).bind(&world);
        let children = bound.nav().children::<ChildRefs>();
        assert!(children.is_empty());

        // Test with EntityPtr - no ChildRefs component
        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(entity);
        let children = ptr.nav_many().children::<ChildRefs>();
        assert!(children.is_empty());
    }
}
