# Macro Fixture Test Coverage

## Coverage Matrix

| Fixture Constant | Test Function | Status |
|-----------------|---------------|---------|
| `PASTE_MACRO_PATTERN` | `test_paste_macro_expansion()` | ✅ Tested |
| `NESTED_PASTE_MACRO` | `test_nested_paste_macro()` | ✅ Tested |
| `MACRO_RULES_PATTERNS` | `test_macro_rules_patterns()` | ✅ Tested |
| `STDLIB_MACROS` | `test_stdlib_macros()` | ✅ Tested |
| `ASYNC_AND_KAMEO_MACROS` | ❌ MISSING | ⚠️ No test |
| `DERIVE_MACROS` | `test_derive_macros()` | ✅ Tested |
| `ATTRIBUTE_MACROS` | ❌ MISSING | ⚠️ No test |
| `LOGGING_MACROS` | `test_logging_macros()` | ✅ Tested |
| `CUSTOM_TRADING_MACROS` | `test_custom_trading_macros()` | ✅ Tested |
| `MACRO_EXPANSION_RESULTS` | `test_macro_expansion_synthetic_calls()` | ✅ Tested |

## Additional Tests
- `test_paste_generates_all_indicator_calls()` - Tests paste! behavior specifically

## Missing Tests
1. **ASYNC_AND_KAMEO_MACROS** - No test for async/Kameo patterns
2. **ATTRIBUTE_MACROS** - No test for attribute macros

## Summary
- **8 out of 10** fixtures have tests
- **2 fixtures** need tests added