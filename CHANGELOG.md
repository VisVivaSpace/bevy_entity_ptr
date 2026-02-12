# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
