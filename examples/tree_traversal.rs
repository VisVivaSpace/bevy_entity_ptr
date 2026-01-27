//! Tree traversal example demonstrating recursive operations with EntityPtr.
//!
//! This example shows how to:
//! - Build a tree structure using EntityHandle references
//! - Recursively traverse trees to sum values
//! - Find the root of a tree by following parent links
//! - Compute tree depth
//!
//! Run with: `cargo run --example tree_traversal`

use bevy_ecs::prelude::*;
use bevy_entity_ptr::{EntityHandle, EntityPtr, WorldRef};

// Components for our tree structure

#[derive(Component)]
struct Name(&'static str);

#[derive(Component)]
struct Value(i32);

#[derive(Component)]
struct Parent(EntityHandle);

#[derive(Component)]
struct Children(Vec<EntityHandle>);

// Recursive function to sum all values in a subtree
fn sum_tree(node: EntityPtr) -> i32 {
    let my_value = node.get::<Value>().map(|v| v.0).unwrap_or(0);

    let children_sum: i32 = node
        .get::<Children>()
        .map(|c| c.0.iter().map(|h| sum_tree(node.follow_handle(*h))).sum())
        .unwrap_or(0);

    my_value + children_sum
}

// Recursive function to find the root by traversing parent links
fn find_root(node: EntityPtr) -> EntityPtr {
    match node.follow::<Parent, _>(|p| p.0) {
        Some(parent) => find_root(parent),
        None => node,
    }
}

// Recursive function to compute tree depth
fn tree_depth(node: EntityPtr) -> usize {
    node.get::<Children>()
        .map(|c| {
            c.0.iter()
                .map(|h| tree_depth(node.follow_handle(*h)))
                .max()
                .unwrap_or(0)
                + 1
        })
        .unwrap_or(0)
}

// Recursive function to collect all node names in pre-order
fn collect_names(node: EntityPtr, names: &mut Vec<&'static str>) {
    if let Some(name) = node.get::<Name>() {
        names.push(name.0);
    }

    if let Some(children) = node.get::<Children>() {
        for child_handle in &children.0 {
            collect_names(node.follow_handle(*child_handle), names);
        }
    }
}

fn main() {
    let mut world = World::new();

    // Build a tree:
    //
    //           root (10)
    //          /    \
    //       a (5)   b (3)
    //         |
    //       c (2)
    //         |
    //       d (7)
    //
    println!("Building tree structure...\n");

    let d = world.spawn((Name("d"), Value(7))).id();
    let c = world
        .spawn((Name("c"), Value(2), Children(vec![EntityHandle::new(d)])))
        .id();
    // Add parent link to d
    world.entity_mut(d).insert(Parent(EntityHandle::new(c)));

    let a = world
        .spawn((Name("a"), Value(5), Children(vec![EntityHandle::new(c)])))
        .id();
    world.entity_mut(c).insert(Parent(EntityHandle::new(a)));

    let b = world.spawn((Name("b"), Value(3))).id();

    let root = world
        .spawn((
            Name("root"),
            Value(10),
            Children(vec![EntityHandle::new(a), EntityHandle::new(b)]),
        ))
        .id();
    world.entity_mut(a).insert(Parent(EntityHandle::new(root)));
    world.entity_mut(b).insert(Parent(EntityHandle::new(root)));

    // SAFETY: world outlives all EntityPtr usage in this scope
    let w = unsafe { WorldRef::new(&world) };

    // Demonstrate tree operations
    let root_ptr = w.entity(root);

    // 1. Sum all values
    let total = sum_tree(root_ptr);
    println!("Sum of all values in tree: {}", total);
    println!("  Expected: 10 + 5 + 2 + 7 + 3 = 27");
    assert_eq!(total, 27);

    // 2. Find root from any node
    let d_ptr = w.entity(d);
    let found_root = find_root(d_ptr);
    let root_name = found_root.get::<Name>().unwrap().0;
    println!("\nFinding root from node 'd': {}", root_name);
    assert_eq!(root_name, "root");

    let b_ptr = w.entity(b);
    let found_root = find_root(b_ptr);
    let root_name = found_root.get::<Name>().unwrap().0;
    println!("Finding root from node 'b': {}", root_name);
    assert_eq!(root_name, "root");

    // 3. Compute tree depth (number of edges on longest path)
    let depth = tree_depth(root_ptr);
    println!("\nTree depth from root: {}", depth);
    println!("  Expected: 3 (root->a, a->c, c->d = 3 edges)");
    assert_eq!(depth, 3);

    let a_ptr = w.entity(a);
    let a_depth = tree_depth(a_ptr);
    println!("Subtree depth from 'a': {}", a_depth);
    assert_eq!(a_depth, 2);

    // 4. Collect all names in pre-order traversal
    let mut names = Vec::new();
    collect_names(root_ptr, &mut names);
    println!("\nPre-order traversal: {:?}", names);
    assert_eq!(names, vec!["root", "a", "c", "d", "b"]);

    println!("\nAll assertions passed!");
}
