use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::world::World;
use bevy_entity_ptr::{BoundEntity, EntityHandle, EntityPtr, WorldExt};
use criterion::{Criterion, black_box, criterion_group, criterion_main};

#[derive(Component)]
struct Parent(EntityHandle);

#[derive(Component)]
struct Value(i32);

#[derive(Component)]
struct Children(Vec<EntityHandle>);

/// Build a linear chain of `depth` entities, each pointing to the next via Parent.
/// Returns the first entity in the chain.
fn build_chain(world: &mut World, depth: usize) -> Entity {
    let mut current = world.spawn(Value(depth as i32)).id();
    for i in (0..depth).rev() {
        let parent = current;
        current = world
            .spawn((Value(i as i32), Parent(EntityHandle::new(parent))))
            .id();
    }
    current
}

/// Build a balanced binary tree of given depth. Returns the root.
fn build_tree(world: &mut World, depth: usize) -> Entity {
    if depth == 0 {
        return world.spawn(Value(1)).id();
    }
    let left = build_tree(world, depth - 1);
    let right = build_tree(world, depth - 1);
    world
        .spawn((
            Value(1),
            Children(vec![EntityHandle::new(left), EntityHandle::new(right)]),
        ))
        .id()
}

// =========================================================================
// EntityPtr vs raw world.get() — linear chain traversal
// =========================================================================

fn traverse_chain_entityptr(world: &World, start: Entity, depth: usize) -> i32 {
    let ptr = world.entity_ptr(start);
    let mut current = ptr;
    for _ in 0..depth {
        current = match current.follow::<Parent, _>(|p| p.0) {
            Some(next) => next,
            None => break,
        };
    }
    current.get::<Value>().map(|v| v.0).unwrap_or(0)
}

fn traverse_chain_raw(world: &World, start: Entity, depth: usize) -> i32 {
    let mut current = start;
    for _ in 0..depth {
        current = match world.get::<Parent>(current) {
            Some(p) => p.0.entity(),
            None => break,
        };
    }
    world.get::<Value>(current).map(|v| v.0).unwrap_or(0)
}

fn traverse_chain_bound(world: &World, start: Entity, depth: usize) -> i32 {
    let mut current = EntityHandle::new(start).bind(world);
    for _ in 0..depth {
        current = match current.follow::<Parent, _>(|p| p.0) {
            Some(next) => next,
            None => break,
        };
    }
    current.get::<Value>().map(|v| v.0).unwrap_or(0)
}

// =========================================================================
// Recursive tree sum
// =========================================================================

fn sum_tree_entityptr(ptr: EntityPtr) -> i32 {
    let mine = ptr.get::<Value>().map(|v| v.0).unwrap_or(0);
    let children_sum: i32 = ptr
        .get::<Children>()
        .map(|c| {
            c.0.iter()
                .map(|h| sum_tree_entityptr(ptr.follow_handle(*h)))
                .sum()
        })
        .unwrap_or(0);
    mine + children_sum
}

fn sum_tree_bound(bound: BoundEntity) -> i32 {
    let mine = bound.get::<Value>().map(|v| v.0).unwrap_or(0);
    let children_sum: i32 = bound
        .get::<Children>()
        .map(|c| {
            c.0.iter()
                .map(|h| sum_tree_bound(h.bind(bound.world())))
                .sum()
        })
        .unwrap_or(0);
    mine + children_sum
}

fn sum_tree_raw(world: &World, entity: Entity) -> i32 {
    let mine = world.get::<Value>(entity).map(|v| v.0).unwrap_or(0);
    let children_sum: i32 = world
        .get::<Children>(entity)
        .map(|c| c.0.iter().map(|h| sum_tree_raw(world, h.entity())).sum())
        .unwrap_or(0);
    mine + children_sum
}

// =========================================================================
// Benchmarks
// =========================================================================

fn bench_chain_traversal(c: &mut Criterion) {
    let mut group = c.benchmark_group("chain_traversal");

    for depth in [5, 10, 50, 100] {
        let mut world = World::new();
        let start = build_chain(&mut world, depth);

        group.bench_function(format!("entityptr_depth_{}", depth), |b| {
            b.iter(|| traverse_chain_entityptr(&world, black_box(start), depth))
        });

        group.bench_function(format!("raw_depth_{}", depth), |b| {
            b.iter(|| traverse_chain_raw(&world, black_box(start), depth))
        });

        group.bench_function(format!("bound_depth_{}", depth), |b| {
            b.iter(|| traverse_chain_bound(&world, black_box(start), depth))
        });
    }

    group.finish();
}

fn bench_tree_sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_sum");

    for depth in [4, 6, 8, 10] {
        let mut world = World::new();
        let root = build_tree(&mut world, depth);
        let node_count = (1 << (depth + 1)) - 1;

        group.bench_function(
            format!("entityptr_depth_{}_nodes_{}", depth, node_count),
            |b| b.iter(|| sum_tree_entityptr(world.entity_ptr(black_box(root)))),
        );

        group.bench_function(format!("bound_depth_{}_nodes_{}", depth, node_count), |b| {
            b.iter(|| sum_tree_bound(world.bind_entity(black_box(root))))
        });

        group.bench_function(format!("raw_depth_{}_nodes_{}", depth, node_count), |b| {
            b.iter(|| sum_tree_raw(&world, black_box(root)))
        });
    }

    group.finish();
}

criterion_group!(benches, bench_chain_traversal, bench_tree_sum);
criterion_main!(benches);
