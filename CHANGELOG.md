# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

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
