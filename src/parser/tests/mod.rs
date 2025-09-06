// Parser regression tests based on actual trading-backend-poc patterns
// These tests ensure we don't break existing parsing functionality

pub mod fixtures;

mod function_parsing;
mod type_parsing;
mod trait_parsing;
mod impl_parsing;
mod macro_expansion;
mod actor_detection;
mod message_detection;
mod call_detection;
mod generic_parsing;
mod async_parsing;
mod operator_parsing;
mod reference_parsing;
mod default_pattern;
mod instance_method_resolution;
mod test_trait_method_field_usage;

// Re-export test runner for use in integration tests
pub use function_parsing::test_function_parsing;
pub use type_parsing::test_type_parsing;
pub use trait_parsing::test_trait_parsing;
pub use impl_parsing::test_impl_parsing;
pub use macro_expansion::{
    test_paste_macro_expansion,
    test_nested_paste_macro,
    test_stdlib_macros,
    test_logging_macros,
    test_derive_macros,
    test_custom_trading_macros,
    test_macro_expansion_synthetic_calls,
    test_paste_generates_all_indicator_calls,
    test_macro_rules_patterns,
    test_async_and_kameo_macros,
    test_attribute_macros
};
pub use actor_detection::test_actor_detection;
pub use message_detection::test_kameo_message_detection;
pub use call_detection::test_call_detection;
pub use generic_parsing::test_generic_parsing;
pub use async_parsing::test_async_parsing;
pub use operator_parsing::test_operator_parsing;
pub use reference_parsing::test_reference_parsing;
pub use default_pattern::test_default_impl_pattern;

/// Run all parser regression tests
pub fn run_all_parser_tests() {
    println!("Running parser regression tests...");
    
    test_function_parsing();
    test_type_parsing();
    test_trait_parsing();
    test_impl_parsing();
    test_paste_macro_expansion();
    test_nested_paste_macro();
    test_stdlib_macros();
    test_custom_trading_macros();
    test_async_and_kameo_macros();
    test_attribute_macros();
    test_actor_detection();
    test_kameo_message_detection();
    test_call_detection();
    test_generic_parsing();
    test_async_parsing();
    test_operator_parsing();
    test_reference_parsing();
    test_default_impl_pattern();
    
    println!("All parser regression tests passed!");
}
