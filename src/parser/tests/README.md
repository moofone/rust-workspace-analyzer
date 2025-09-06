# Parser Test Suite

## Overview
This comprehensive test suite validates the Rust parser functionality against real-world patterns from the trading-backend-poc workspace.

## Test Coverage

### Core Parsing Tests
- **function_parsing.rs**: Tests regular functions, async functions, methods, generics
- **type_parsing.rs**: Tests structs, enums, type aliases, visibility modifiers
- **trait_parsing.rs**: Tests trait definitions, associated types, generic traits
- **impl_parsing.rs**: Tests impl blocks, trait implementations, operator overloading

### Advanced Features
- **generic_parsing.rs**: Tests generic types, functions, const generics, lifetime parameters  
- **async_parsing.rs**: Tests async functions, async traits, async blocks, tokio spawns
- **operator_parsing.rs**: Tests operator overloading (critical for Pf64 patterns)
- **macro_expansion.rs**: Tests macro detection, synthetic call generation

### Framework Integration
- **actor_detection.rs**: Tests Kameo actor patterns, distributed actors
- **message_detection.rs**: Tests message types, handlers, distributed message flows
- **call_detection.rs**: Tests function calls, method calls, cross-crate calls
- **reference_parsing.rs**: Tests cross-module references, type references

## Running Tests

### Run all parser tests
```bash
cargo test --lib parser::tests
```

### Run specific test module
```bash
cargo test --lib parser::tests::function_parsing
```

### Run with output
```bash
cargo test --lib parser::tests -- --nocapture
```

## Key Test Patterns

### Critical for trading-backend-poc
1. **Operator Overloading**: Tests f64 + Pf64 patterns that were showing false positives
2. **Macro Expansion**: Tests define_indicator_enums! and distributed_actor! macros
3. **Actor Systems**: Tests both local and distributed actor patterns
4. **Cross-crate Calls**: Tests trading-ta, trading-core, trading-strategy interactions
