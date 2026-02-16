# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.6.0] - 2026-02-16

### Breaking Changes
- `HasParent::parent_entity()` renamed to `HasParent::parent_handle()`, now returns `Option<EntityHandle>` instead of `Option<Entity>`
- `HasChildren::children_entities()` renamed to `HasChildren::children_handles()`, now returns `&[EntityHandle]` instead of `&[Entity]`
- `BoundEntityNav::children()` and `EntityPtrNavMany::children()` now return `impl Iterator` instead of `Vec`

### Added
- `Display` impl for `EntityHandle`
- `Clone` + `Copy` derives on `WorldRef`
- `PartialEq`, `Eq`, and `Hash` implementations for `BoundEntity` (entity-only comparison, matching `EntityPtr`)
- Criterion benchmarks for traversal performance (`benches/traversal.rs`)
- GitHub Actions CI (test, clippy, fmt, MSRV, Miri) with cargo build caching

### Changed
- Removed redundant `unsafe impl Send/Sync` on `EntityHandle` (auto-derived from `Entity`)
- Expanded safety documentation on `WorldExt::entity_ptr()` soundness invariant
- README rewritten: frames `bevy_ecs` as a general ECS library, not game-specific; emphasizes ergonomic `WorldExt` interface as the primary API with documented safety tradeoffs
- Simplified doc examples: removed misleading system signatures, focused on API demonstration
- All doc examples now compile-checked (changed from `ignore` to `no_run` or fully runnable)

## [0.5.0] - 2026-02-11

### Changed
- Updated to Bevy 0.18 compatibility
- No API changes required — all production and test code compatible as-is

## [0.4.0] - 2026-02-11

### Changed
- Updated to Bevy 0.17 compatibility
- Replaced deprecated `iter_entities()` with query-based iteration in examples and docs
- Updated `Entity::from_raw()` to `Entity::from_raw_u32()` in tests (Bevy 0.17 API change)

## [0.3.0] - 2026-02-11

### Changed
- Updated to Bevy 0.16 compatibility
- Updated to Rust edition 2024 (`rust-version = "1.85"`)
- Added `authors`, `homepage`, `documentation` fields to Cargo.toml
- Added `[package.metadata.docs.rs]` section
- Fixed README license line to match LICENSE file (MIT)
- Added Bevy compatibility table to README
- Added `llm-context.md` for LLM integration guidance

## [0.2.0] - 2026-02-03

### Added
- `WorldExt` extension trait providing `entity_ptr()` and `bind_entity()` methods on `World`
- `PartialEq`, `Eq`, and `Hash` implementations for `EntityPtr` (compares entity field only)
- "Choosing Between Types" documentation section with safety/ergonomics guidance

### Changed
- Users no longer need to write `unsafe` blocks to create `EntityPtr` - use `world.entity_ptr(entity)` instead

## [0.1.0] - 2026-01-26

### Added
- `EntityHandle` - lightweight 8-byte handle safe for component storage (Send + Sync)
- `BoundEntity<'w>` - scoped fluent access with explicit world parameter
- `WorldRef` - system entry point for ergonomic EntityPtr approach
- `EntityPtr` - 16-byte smart pointer with embedded world reference
- `follow()` and `follow_opt()` methods for entity graph traversal
- `follow_handle()` for recursive tree patterns
- `HasParent` and `HasChildren` navigation traits (feature-gated)
- Comprehensive documentation and examples
- Three runnable examples: tree_traversal, entity_graph, mixed_usage
