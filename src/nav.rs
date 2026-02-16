//! Navigation traits for parent/children traversal.
//!
//! This module provides traits for components that define entity relationships.
//! Feature-gated behind `nav-traits`.

use crate::handle::EntityHandle;

/// Trait for components that reference a parent entity.
///
/// Implement this on your component type to enable `.nav().parent()` navigation.
///
/// # Example
/// ```no_run
/// use bevy_ecs::prelude::*;
/// use bevy_entity_ptr::{EntityHandle, HasParent};
///
/// #[derive(Component)]
/// struct Parent(EntityHandle);
///
/// impl HasParent for Parent {
///     fn parent_handle(&self) -> Option<EntityHandle> {
///         Some(self.0)
///     }
/// }
/// ```
pub trait HasParent {
    /// Returns a handle to the parent entity, if one exists.
    fn parent_handle(&self) -> Option<EntityHandle>;
}

/// Trait for components that reference child entities.
///
/// Implement this on your component type to enable `.nav_many().children()` navigation.
///
/// # Example
/// ```no_run
/// use bevy_ecs::prelude::*;
/// use bevy_entity_ptr::{EntityHandle, HasChildren};
///
/// #[derive(Component)]
/// struct Children(Vec<EntityHandle>);
///
/// impl HasChildren for Children {
///     fn children_handles(&self) -> &[EntityHandle] {
///         &self.0
///     }
/// }
/// ```
pub trait HasChildren {
    /// Returns a slice of handles to child entities.
    fn children_handles(&self) -> &[EntityHandle];
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
            c.parent_handle()
                .map(|h| BoundEntity::new(h.entity(), self.0.world()))
        })
    }

    /// Navigates to child entities using a component that implements `HasChildren`.
    ///
    /// Returns an iterator of `BoundEntity` for each child. Returns an empty
    /// iterator if the component is missing.
    #[inline]
    pub fn children<T: bevy_ecs::component::Component + HasChildren>(
        self,
    ) -> impl Iterator<Item = BoundEntity<'w>> + 'w {
        self.0.get::<T>().into_iter().flat_map(move |c| {
            c.children_handles()
                .iter()
                .copied()
                .map(move |h| BoundEntity::new(h.entity(), self.0.world()))
        })
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
        self.0.get::<T>().and_then(|c| {
            c.parent_handle()
                .map(|h| EntityPtr::new(h.entity(), self.0.world()))
        })
    }
}

impl EntityPtrNavMany {
    /// Navigates to child entities using a component that implements `HasChildren`.
    ///
    /// Returns an iterator of `EntityPtr` for each child. Returns an empty
    /// iterator if the component is missing.
    #[inline]
    pub fn children<T: bevy_ecs::component::Component + HasChildren>(
        self,
    ) -> impl Iterator<Item = EntityPtr> {
        self.0.get::<T>().into_iter().flat_map(move |c| {
            c.children_handles()
                .iter()
                .copied()
                .map(move |h| EntityPtr::new(h.entity(), self.0.world()))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ptr::WorldRef;
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

    #[test]
    fn bound_entity_nav_parent() {
        let mut world = World::new();
        let parent = world.spawn(Name("parent")).id();
        let child = world
            .spawn((Name("child"), ParentRef(Some(EntityHandle::new(parent)))))
            .id();

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
            .spawn((
                Name("parent"),
                ChildRefs(vec![EntityHandle::new(child1), EntityHandle::new(child2)]),
            ))
            .id();

        let bound = EntityHandle::new(parent).bind(&world);
        let children: Vec<_> = bound.nav().children::<ChildRefs>().collect();

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
        let child = world
            .spawn((Name("child"), ParentRef(Some(EntityHandle::new(parent)))))
            .id();

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
            .spawn((
                Name("parent"),
                ChildRefs(vec![EntityHandle::new(child1), EntityHandle::new(child2)]),
            ))
            .id();

        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(parent);
        let children: Vec<_> = ptr.nav_many().children::<ChildRefs>().collect();

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
        assert_eq!(bound.nav().children::<ChildRefs>().count(), 0);

        // Test with EntityPtr
        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(parent);
        assert_eq!(ptr.nav_many().children::<ChildRefs>().count(), 0);
    }

    /// Test nav_many().children() returns empty iterator when component is missing.
    #[test]
    fn nav_many_no_children_component() {
        let mut world = World::new();
        let entity = world.spawn(Name("no_children")).id();

        // Test with BoundEntity - no ChildRefs component
        let bound = EntityHandle::new(entity).bind(&world);
        assert_eq!(bound.nav().children::<ChildRefs>().count(), 0);

        // Test with EntityPtr - no ChildRefs component
        // SAFETY: world outlives usage
        let world_ref = unsafe { WorldRef::new(&world) };
        let ptr = world_ref.entity(entity);
        assert_eq!(ptr.nav_many().children::<ChildRefs>().count(), 0);
    }
}
