# Examples

Run any example with `cargo run --example <name>`.

## tree_traversal

Recursive tree operations with `EntityPtr`: sum values across a tree, find the root from any node, compute depth, and collect names in pre-order. Start here to see the core recursive pattern.

## entity_graph

Non-tree entity relationships: inventory systems, optional references, and complex graph navigation using `follow` and `follow_opt`.

## mixed_usage

Shows both API approaches side by side: `EntityHandle`/`BoundEntity` (fully safe, explicit `&World`) and `EntityPtr` (ergonomic, no lifetime threading). Demonstrates converting between them and handling stale references after despawn.

## concurrent_systems

Multi-threaded read-only access using Bevy's scheduler. Two systems run in parallel, each creating independent `EntityPtr` instances from `&World`. Demonstrates why `!Send` on `EntityPtr` is safe for concurrent reads.
