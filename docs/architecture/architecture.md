# Rust Workspace Analyzer — Architecture

This document explains the architecture of the Rust workspace analyzer with a focus on the parser, pattern detection, and how core components interact across parsing, call resolution, actor detection, and graph population.

## Detected Patterns

- Actor Implementations: impl blocks of `Actor` ('impl Actor for Type').
- Actor Spawns:
  - Direct type calls: `SomeActor::spawn(...)`, `::spawn_with_mailbox`, `::spawn_link`, `::spawn_in_thread`, `::spawn_with_storage`.
  - Trait method: `Actor::spawn(instance)`.
  - Module function: `kameo::actor::spawn(instance)`.
  - Non-actor frameworks filtered out (e.g., `tokio::spawn`, `std::thread::spawn`, `async_std::task::spawn`).
- ActorRef Tracking:
  - Struct fields with `ActorRef<T>` (with or without wrapper like `Option<ActorRef<T>>`).
  - Let-bindings declaring `ActorRef<T>`.
  - Assignments from spawn calls, variable-to-type inference (with name-based heuristics).
- Message Types:
  - Structs/enums ending with `Tell`, `Ask`, `Message`, or `Query`.
- Message Handlers:
  - `impl Message<T> for Type { type Reply = ...; async fn ... }`.
  - Links handlers back to actors' `local_messages`.
- Message Sends:
  - `actor_ref.tell(msg)` and `actor_ref.ask(msg)` detection, including chained calls and message type extraction from struct expressions or variable types.
- Distributed Actors and Flows:
  - `kameo::distributed_actor!(ActorName, { MessageType ... })` macro detection and aggregation per actor.
  - Distributed message flows: detection of `.tell`/`.ask` on distributed refs with sender context and location.
- Functions, Types, Impl Blocks:
  - Top-level functions and impl methods (inherent and trait impl methods).
- Function Calls:
  - Direct, method, associated, generic, and macro invocations; `Self::` resolution to concrete type within impl context.
- Imports and Modules:
  - `use` forms (simple, grouped, glob, module) and `mod` declarations.
- Macro Expansions (synthetic calls):
  - `paste!` macro expansions for trading indicators; synthetic calls generated to modeled indicator types and variants (`Input`, `Output`, `TrendOutput`, `UnifiedOutput`).
- Framework Dispatch (synthetic calls):
  - Synthetic calls to model runtime trait method dispatch for WebSocket actors and Actix lifecycle (in `WorkspaceAnalyzer` via `FrameworkPatterns`).

## Parser Architecture And Flow

- Tree-sitter Frontend:
  - The parser uses `tree_sitter_rust` with a curated `QuerySet` to find functions, types, impls, calls, modules, imports, actors, message types/handlers, ActorRef variables, message sends, and spawn patterns.
- File Parsing (`parse_file` → `parse_source`):
  - Determines test context (`tests/`, `examples/`, benches).
  - Runs extraction passes in sequence:
    - Modules (`mod`), imports (`use`).
    - Functions (excludes functions inside impl; those come from impl parsing).
    - Types (struct/enum/trait/type alias).
    - Impl blocks (trait impl and inherent):
      - All impl methods are parsed as `RustFunction` with `is_trait_impl` set accordingly and added to both `impls` and `functions`.
    - Calls:
      - Captures direct, method, associated, generic, and macro calls.
      - Resolves `Self::foo` to `Type::foo` using surrounding impl context.
      - Macro expansions and synthetic calls for `paste!` indicator patterns.
    - Actors:
      - Primary: explicit `impl Actor for Type`.
      - Secondary: inferred from `impl Message<T> for Type` (inferred actor) to avoid false positives from naming.
      - Inference from derives or `ActorRef<T>` usage is intentionally disabled to reduce noise.
    - Actor Spawns:
      - Direct, trait, and module-based spawn patterns with method classification (`SpawnMethod`) and `SpawnPattern`.
      - Filters non-actor frameworks and test contexts.
    - Distributed Actors and Flows:
      - `kameo::distributed_actor!` macro grouping by actor.
      - `.tell/.ask` on distributed refs with sender/target inference.
    - Message Types and Handlers:
      - Heuristic name matching for message-type declarations.
      - Trait-based handlers with reply type extraction and actor/message linkage.
    - ActorRef Variables:
      - Struct fields and let bindings for `ActorRef<T>`, plus variable-based inference (e.g., stripping `_ref`, snake-to-Pascal heuristics).
  - Deduplication:
    - Functions deduped by `qualified_name + line_start`.
    - Types deduped similarly.
    - Actors deduped by `(name, crate)`.
    - Spawns deduped by `(parent, child, file, line)`.
- Reference Resolution:
  - `ReferenceResolver` builds a symbol table combining qualified and simple names (adds crate-qualified variants for `crate::` paths).
  - Import table per file, supporting simple/grouped/module imports (glob deferred).
  - Resolves calls by:
    - Import-based resolution for local aliases.
    - Fallbacks across `module::name`, `crate::name`, and simple `name`.
    - Qualified and method call handling; `Self::` already resolved at parse.
  - Sets `qualified_callee`, `to_crate`, `cross_crate` on `FunctionCall`.
- Linkage:
  - `link_message_handlers_to_actors` merges handler data back into the actor's `local_messages`.

## Core Components

- RustParser:
  - Orchestrates parsing per file; builds `ParsedSymbols`.
  - Encodes robust extraction with tree-sitter queries and heuristics/filters to reduce false positives.
- QuerySet:
  - Centralizes tree-sitter queries: functions, types, impls, calls, imports, modules, actors, spawns, message types, handlers, ActorRef variables, message sends.
- ParsedSymbols:
  - Data model aggregating symbols across all extraction passes (functions, types, impls, modules, imports, calls, actors, spawns, message types/handlers, message sends, distributed entities, macro expansions).
  - Offers merging and simple lookups.
- ReferenceResolver:
  - Builds symbol and import tables; resolves call targets to qualified names.
  - Provides synthetic trait call generation logic (now delegated to framework patterns for runtime dispatch).
- TraitIndex:
  - Indexes type methods and trait implementations; resolves method to function across inherent/trait/UFCS contexts.
  - Supports introspection utilities for analysis stages that require robust type/trait knowledge.
- WorkspaceDiscovery:
  - Uses `cargo_metadata` to enumerate crates (optionally recursive up to depth), classify layers, and filter by config.
- WorkspaceAnalyzer:
  - High-level orchestration:
    - Discovers crates to analyze.
    - Parses each crate with `RustParser` to build a workspace snapshot.
    - Resolves references and adds synthetic calls via framework patterns.
    - Optional: builds global symbol index for cross-crate resolution with caching.
    - Optional: generates embeddings and populates Memgraph.
- FrameworkPatterns:
  - Regex-based patterns for frameworks (Tokio, Actix-Web, async-std, WebSocket).
  - Adds synthetic calls for runtime dispatch that is not statically visible (e.g., lifecycle and WebSocket message handlers).
- GlobalSymbolIndex:
  - Aggregates functions, types, traits, exports across crates with caching.
  - Used for cross-crate resolution of `Type::method` when local context is insufficient.
- MemgraphClient and Import Pipeline:
  - Efficient, batched import with `ANALYTICAL` mode and index creation.
  - Populates nodes (modules, types, functions, actors, message types, macro expansions) and relationships (CALLS, IMPLEMENTS, SPAWNS, HANDLES, SENDS).
- ArchitectureAnalyzer:
  - Graph-based checks: layer violations, circular dependencies, dependency direction, public API constraints.
- EmbeddingGenerator and SemanticSearch:
  - Generates deterministic local embeddings for functions and types.
  - Supports semantic querying and similarity.

## System Interactions

- Parse → Resolve → Enrich → Persist:
  - Parser extracts `ParsedSymbols` per file; merges per crate and workspace.
  - ReferenceResolver resolves calls; WorkspaceAnalyzer applies framework-based synthetic calls.
  - GlobalSymbolIndex (optional) adds cross-crate resolutions; EmbeddingGenerator (optional) produces embeddings.
  - MemgraphClient (optional) persists nodes/edges for analysis and visualization.
- Actor System Modeling:
  - Detects actors from `impl Actor` and from `impl Message<T> for Type` (inferred actors); collects spawns; links message handlers; detects message sends; models distributed actors and flows.
- Pattern Matching:
  - QuerySet captures syntax nodes; name-based heuristics refine actors/messages; explicit filters remove non-actor concurrency patterns.
  - Macro expansions (`paste!`) generate synthetic calls to represent code generated at compile time.
- Architecture Checks:
  - Graph model enables layer and dependency validations and public API rules.

## Data Model Highlights

- Function (`RustFunction`):
  - Qualified name, ID, crate/module/file/line metadata, visibility, async/unsafe/generics flags, is-test, is-trait-impl, signature, params/return.
- Type (`RustType`):
  - Kind (Struct/Enum/Trait/TypeAlias/Union), fields/variants, visibility, generics, test flag, embedding text.
- Impl (`RustImpl`):
  - `type_name`, optional `trait_name`, full set of parsed `methods`.
- Call (`FunctionCall`):
  - Caller ID/module, callee name, call type (Direct/Method/Associated/Macro), line, cross-crate info, file, synthetic flags and macro context, synthetic confidence.
- Module/Import (`RustModule`, `RustImport`):
  - `mod` definitions; `use` forms (simple/grouped/glob/module) with per-file import tables.
- Actor (`RustActor`):
  - Name/qualified name, crate/module/file/lines, `ActorImplementationType` (Local/Distributed/Supervisor/Unknown), local messages, inferred flag.
- Actor Spawn (`ActorSpawn`):
  - Parent/child link, method (`SpawnMethod`), pattern (`SpawnPattern`), context (function), args, line/file crate-to-crate info.
- Message Type/Handler (`MessageType`, `MessageHandler`):
  - Message kind (`Tell/Ask/Message/Query`) and handler with reply type, async flag, file/line/crate metadata.
- Message Send (`MessageSend`):
  - Sender/receiver with optional qualified names, message type, send method (`Tell`/`Ask`), location, crate info.
- Distributed (`DistributedActor`, `DistributedMessageFlow`):
  - Actors and flows with message types, sender context, send method, and location.
- Macro (`MacroExpansion` and `MacroContext`):
  - Expansion ID/range/type/pattern, target functions, containing function, and macro context.

## Pattern Matching Details

- Function Extraction:
  - All `function_item` nodes; impl methods parsed via impl walk. Avoids double-counting by skipping functions inside impls during the standalone function pass.
- Type Extraction:
  - Struct/Enum/Trait/Type alias; lightweight to avoid segfault risks (fields/variants captured minimally).
- Impl Extraction:
  - Captures trait name (qualified or generic), type name (including primitives), and methods in `declaration_list`.
  - Methods propagated into `functions` and marked `is_trait_impl`.
- Calls:
  - Patterns for simple identifiers, field-method calls, scoped identifiers (`Type::method`), generic functions, and macros.
  - `Self::` remapped to concrete `Type::method` based on containing impl.
- Actor:
  - Primary: `impl Actor for Type`.
  - Inferred from `impl Message<T> for Type` for more complete coverage.
  - Derive and `ActorRef<T>`-based inference disabled to reduce false positives.
- Actor Spawns:
  - Direct type, trait method, and module function patterns; filters non-actor spawn APIs; test-context filtered; context detection maps spawns to lifecycle or handler contexts.
- Message Types and Handlers:
  - Type names ending with `Tell|Ask|Message|Query`.
  - `impl Message<T> for Type` with `Reply` extraction; links back to actors.
- Message Sends:
  - `.tell`/`.ask` on field or local `ActorRef<T>` with message type extracted from struct expressions or var type tracking.
- Distributed:
  - `kameo::distributed_actor!` macro for actor + message lists.
  - Flows captured by `.tell/.ask` against distributed refs with sender/target inference.
- Macro Expansions:
  - `paste!` expansions recorded as range-bearing nodes with context and synthetic calls to indicator types/methods, using an indicator registry and name-conversion heuristics.
- Framework Patterns:
  - Regex-based detection for Actix and WebSocket runtime dispatch; injects synthetic calls to trait methods (e.g., `event_stream`, `handle_message`, lifecycle methods) to reflect runtime behavior not statically evident.

## Call Resolution Pipeline

- Symbol Table:
  - Stores qualified and simple names for functions and types; adds crate-qualified variants for `crate::` paths.
- Import Table:
  - Per-file rewrite of local aliases to fully qualified names; supports `use` simple/grouped/module (glob deferred).
- Resolution Strategy:
  - Qualified references resolved directly or by pattern-based fallback.
  - Simple/method references resolved against imports, then `module::name` and `crate::name` candidates.
  - Updates `FunctionCall` with `qualified_callee`, `to_crate`, and `cross_crate`.
- Cross-Crate Enrichment:
  - `GlobalSymbolIndex` can resolve `Type::method` calls across crates.
  - `FrameworkPatterns` add synthetic calls to represent trait-based runtime dispatch.

## Key Interactions

- WorkspaceAnalyzer integrates:
  - WorkspaceDiscovery → per-crate parse via RustParser → reference resolution → synthetic framework calls → optional global index, embeddings, and graph population.
- MemgraphClient import pipeline:
  - Creates indexes, switches to analytical mode, imports nodes then relationships in batches, switches back to transactional mode.
- ArchitectureAnalyzer runs graph queries:
  - Layer violations, circular dependencies, directionality, and public API access.

## Heuristics and Safeguards

- False-Positive Avoidance:
  - Disabled actor inference from derives and `ActorRef<T>` usage; explicit filters for non-actor frameworks' `spawn`.
- Contextual Inference:
  - Function containment used for `Self::` resolution and spawn context classification.
  - Variable names with `_actor/_ref` and snake-to-Pascal conversions for type inference when types are implicit.
- Test Sensitivity:
  - Test file detection; spawns inside test contexts filtered from actor spawn relationships.
- Deduplication:
  - Functions, types, actors, and spawns deduped on stable keys (qualified names + lines).
- Robustness:
  - Conservative field/variant parsing for types to avoid parser segfaults; doc extraction suppressed where unstable.
- Distributed Flow Assumptions:
  - Receiver crate currently assumed same as sender, configurable in future refinement.

## Extensibility

- Adding Patterns:
  - Extend `QuerySet` for new syntax or frameworks; add filters to reduce noise.
  - Add `FrameworkPatterns` entries for new runtimes and dispatch styles; compile and validate regex complexity.
- Cross-Crate Improvements:
  - Expand `GlobalSymbolIndex` and resolution in `WorkspaceAnalyzer` to cover more dispatch patterns and trait object calls.
- Graph Model:
  - Add new node/edge types or properties in `MemgraphClient` import pipeline and cypher migrations.
- Semantic Search:
  - Plug in different embedding backends or enrich embedding generation for richer developer workflows.