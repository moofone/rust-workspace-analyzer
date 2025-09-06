use crate::parser::RustParser;
use std::path::Path;

/// Ensure instance method calls (obj.method()) are not given a qualified_callee yet.
/// Example adapted from trading-backend-poc docs:
///   let result = ema.update(Pf64(50000.0));
#[test]
fn test_instance_method_unresolved_qualified_callee() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
pub struct Ema;
impl Ema { pub fn new() -> Self { Ema } pub fn update(&mut self, _v: Pf64) -> Pf64 { _v } }
pub struct Pf64(f64);

pub fn demo() {
    let mut ema = Ema::new();
    let result = ema.update(Pf64(50000.0));
    let _ = result;
}
"#;

    let result = parser
        .parse_source(source, Path::new("test.rs"), "test_crate")
        .expect("parse failed");

    let mut found = false;
    for call in &result.calls {
        if call.callee_name == "update" {
            assert!(call.qualified_callee.is_none(), "qualified_callee should be None for instance methods, got {:?}", call.qualified_callee);
            found = true;
            break;
        }
    }
    assert!(found, "Did not detect ema.update(..) call");
}

/// Ensure Type::new() (scoped calls) are not given a qualified_callee yet.
/// Based on exact usage from trading-backend-poc:
///   let mut pattern = DiscreteAggregationPattern::new(10);
#[test]
fn test_scoped_type_method_unresolved_qualified_callee() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
pub struct DiscreteAggregationPattern;
impl DiscreteAggregationPattern {
    pub fn new(_capacity: usize) -> Self { DiscreteAggregationPattern }
}

pub fn demo() {
    let mut pattern = DiscreteAggregationPattern::new(10);
    let _ = &mut pattern;
}
"#;

    let result = parser
        .parse_source(source, Path::new("test.rs"), "test_crate")
        .expect("parse failed");

    let mut found = false;
    for call in &result.calls {
        if call.callee_name == "new" {
            assert!(call.qualified_callee.is_none(), "qualified_callee should be None for scoped type methods, got {:?}", call.qualified_callee);
            found = true;
            break;
        }
    }
    assert!(found, "Did not detect DiscreteAggregationPattern::new(10) call");
}

/// Verify that Type::new() becomes resolved after running the reference resolver.
#[test]
fn test_scoped_type_method_resolves_with_reference_resolver() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
pub struct DiscreteAggregationPattern;
impl DiscreteAggregationPattern {
    pub fn new(_capacity: usize) -> Self { DiscreteAggregationPattern }
}

pub fn demo() {
    let mut pattern = DiscreteAggregationPattern::new(10);
    let _ = &mut pattern;
}
"#;

    let mut parsed = parser
        .parse_source(source, Path::new("test.rs"), "test_crate")
        .expect("parse failed");

    let mut found = false;
    for call in &parsed.calls {
        if call.callee_name == "new" {
            assert!(call.qualified_callee.is_none(), "qualified_callee should be None before resolution, got {:?}", call.qualified_callee);
            found = true;
            break;
        }
    }
    assert!(found, "Did not detect DiscreteAggregationPattern::new(10) call");

    crate::parser::references::resolve_all_references(&mut parsed).expect("resolver failed");

    let mut resolved_found = false;
    for call in &parsed.calls {
        if call.callee_name == "new" {
            assert!(call.qualified_callee.is_some());
            resolved_found = true;
            break;
        }
    }
    assert!(resolved_found, "Resolved call not found");
}

/// Failing guard: MonotonicSeries::new(period, is_max) should remain unresolved even after resolver.
/// Exact-style usage from trading-ta::series::monotonic_series tests.
#[test]
#[ignore = "Documents desired future behavior - Type::method patterns should not be resolved without type inference"]
fn test_monotonic_series_new_unresolved_even_with_resolver() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
pub struct MonotonicSeries;
impl MonotonicSeries { pub fn new(_period: usize, _is_max: bool) -> Self { MonotonicSeries } }

pub fn demo() {
    let mut ms = MonotonicSeries::new(3, true);
    let _ = &mut ms;
}
"#;

    let mut parsed = parser
        .parse_source(source, Path::new("test.rs"), "test_crate")
        .expect("parse failed");

    crate::parser::references::resolve_all_references(&mut parsed).expect("resolver failed");

    let mut found = false;
    for call in &parsed.calls {
        if call.callee_name == "new" {
            assert!(call.qualified_callee.is_none(), "MonotonicSeries::new should be unresolved, got {:?}", call.qualified_callee);
            found = true;
            break;
        }
    }
    assert!(found, "Did not detect MonotonicSeries::new call");
}

/// Failing guard: PSquareQuartiles::new() should remain unresolved even after resolver.
/// Exact usage from trading-ta/test_cumulative_quartiles.rs.
#[test]
#[ignore = "Documents desired future behavior - Type::method patterns should not be resolved without type inference"]
fn test_psquare_quartiles_new_unresolved_even_with_resolver() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
pub struct PSquareQuartiles;
impl PSquareQuartiles { pub fn new() -> Self { PSquareQuartiles } }

pub fn main() {
    let mut p2 = PSquareQuartiles::new();
    let _ = &mut p2;
}

/// Ensure MonotonicSeries::compute instance calls are detected and remain unresolved (no qualified_callee).
/// Mirrors trading_ta::series::monotonic_series compute pattern.
#[test]
fn test_monotonic_series_compute_instance_call_unresolved() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
use std::collections::VecDeque;

pub struct Pf64(pub f64);
impl Pf64 { pub fn na(&self) -> bool { false } }

pub struct MonotonicSeries {
    buffer: VecDeque<Pf64>,
    queue: VecDeque<(Pf64, usize)>,
    period: usize,
    is_max: bool,
    position: usize,
}

impl MonotonicSeries {
    pub fn new(period: usize, is_max: bool) -> Self {
        Self { buffer: VecDeque::with_capacity(period * 2), queue: VecDeque::with_capacity(period), period, is_max, position: 0 }
    }

    pub fn compute(&mut self, value: Pf64) {
        while let Some(&(_, pos)) = self.queue.front() {
            if self.position.wrapping_sub(pos) >= self.period {
                self.queue.pop_front();
            } else {
                break;
            }
        }
        while let Some(&(val, _)) = self.queue.back() {
            if Self::should_pop(val, value, self.is_max) {
                self.queue.pop_back();
            } else {
                break;
            }
        }
        self.queue.push_back((value, self.position));
    }

    fn should_pop(_val: Pf64, _value: Pf64, _is_max: bool) -> bool { false }
}

pub fn demo() {
    let mut ms = MonotonicSeries::new(3, true);
    ms.compute(Pf64(1.0));
}
"#;

    let result = parser
        .parse_source(source, std::path::Path::new("test.rs"), "test_crate")
        .expect("parse failed");

    let mut saw_compute = false;
    for call in &result.calls {
        if call.callee_name == "compute" {
            assert!(call.qualified_callee.is_none(), "qualified_callee should be None for instance compute(), got {:?}", call.qualified_callee);
            saw_compute = true;
        }
    }
    assert!(saw_compute, "Did not detect ms.compute(..) call");
}

/// Instance method on an actor ref (found_ref.tell(...).send()) should remain unresolved.
/// Exact snippet adapted from trading-backend-poc/test_kameo_lookup_fix.rs
#[test]
fn test_addr_send_unresolved_qualified_callee() {
    let mut parser = RustParser::new().expect("Failed to create parser");

    let source = r#"
// Test script to verify kameo lookup fix
use kameo::actor::{spawn, ActorRef};
use kameo::{Actor, Message};
use anyhow::Result;

#[derive(Clone)]
struct TestActor;

impl Actor for TestActor {}

#[derive(Clone)]
struct TestMessage;

impl Message<TestMessage> for TestActor {
    type Reply = ();
    
    async fn handle(&mut self, _msg: TestMessage, _ctx: &mut kameo::actor::Context<Self, Self::Reply>) -> Self::Reply {
        println!("Test message received");
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let actor = TestActor;
    let actor_ref = spawn(actor);
    
    match ActorRef::<TestActor>::lookup("test_actor").await {
        Ok(Some(found_ref)) => {
            // Test that it actually works
            found_ref.tell(TestMessage).send().await?;
        }
        _ => {}
    }
    Ok(())
}
"#;

    let result = parser
        .parse_source(source, Path::new("test.rs"), "test_crate")
        .expect("parse failed");

    let mut saw_tell = false;
    let mut saw_send = false;
    for call in &result.calls {
        if call.callee_name == "tell" {
            assert!(call.qualified_callee.is_none(), "qualified_callee should be None for instance tell(), got {:?}", call.qualified_callee);
            saw_tell = true;
        }
        if call.callee_name == "send" {
            assert!(call.qualified_callee.is_none(), "qualified_callee should be None for instance send(), got {:?}", call.qualified_callee);
            saw_send = true;
        }
    }
    assert!(saw_tell, "Did not detect found_ref.tell(..) call");
    assert!(saw_send, "Did not detect addr.send(..) call");
}
