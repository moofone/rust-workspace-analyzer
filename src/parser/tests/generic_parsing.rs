use crate::parser::RustParser;
use std::path::Path;

/// Test generic type and function parsing from trading patterns
#[test]
pub fn test_generic_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case with complex generics from trading patterns
    let source = r#"
use std::marker::PhantomData;

// Generic struct with trait bounds
pub struct TradingEngine<T, U> 
where
    T: TradingStrategy + Send + Sync,
    U: DataProvider,
{
    strategy: T,
    provider: U,
    _phantom: PhantomData<(T, U)>,
}

// Generic with lifetime parameters
pub struct DataCache<'a, T: Clone> {
    data: &'a [T],
    cache: Vec<T>,
}

// Generic enum
pub enum Result<T, E = Error> {
    Ok(T),
    Err(E),
}

// Generic function with multiple bounds
pub fn process<'a, T, F>(data: &'a [T], processor: F) -> Vec<T>
where
    T: Clone + Send + 'a,
    F: Fn(&T) -> bool + Send + Sync,
{
    data.iter()
        .filter(|item| processor(*item))
        .cloned()
        .collect()
}

// Generic impl block
impl<T, U> TradingEngine<T, U>
where
    T: TradingStrategy + Send + Sync,
    U: DataProvider,
{
    pub fn new(strategy: T, provider: U) -> Self {
        Self {
            strategy,
            provider,
            _phantom: PhantomData,
        }
    }
    
    pub fn execute<R>(&mut self, input: R) -> Result<(), Error>
    where
        R: Into<TradingSignal>,
    {
        Ok(())
    }
}

// Generic type alias
pub type BoxedStrategy<T> = Box<dyn TradingStrategy<Item = T> + Send + Sync>;

// Associated type in trait
pub trait Container {
    type Item;
    
    fn get(&self) -> &Self::Item;
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check generic struct parsing
    let engine = result.types.iter()
        .find(|t| t.name == "TradingEngine")
        .expect("Should find TradingEngine");
    
    assert!(engine.is_generic, "TradingEngine should be marked as generic");
    
    // Check struct with lifetime
    let cache = result.types.iter()
        .find(|t| t.name == "DataCache")
        .expect("Should find DataCache");
    
    assert!(cache.is_generic, "DataCache should be marked as generic");
    
    // Check generic enum with default
    let result_enum = result.types.iter()
        .find(|t| t.name == "Result")
        .expect("Should find Result enum");
    
    assert!(result_enum.is_generic, "Result should be marked as generic");
    
    // Check generic function
    let process_fn = result.functions.iter()
        .find(|f| f.name == "process")
        .expect("Should find process function");
    
    assert!(process_fn.is_generic, "process should be marked as generic");
    
    // Check generic method with additional bounds
    let execute_fn = result.functions.iter()
        .find(|f| f.name == "execute")
        .expect("Should find execute method");
    
    assert!(execute_fn.is_generic, "execute should be marked as generic");
}

/// Test const generics parsing
#[test]
fn test_const_generics() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
// Const generic array wrapper
pub struct FixedBuffer<const N: usize> {
    data: [f64; N],
}

impl<const N: usize> FixedBuffer<N> {
    pub fn new() -> Self {
        Self {
            data: [0.0; N],
        }
    }
    
    pub fn get(&self, index: usize) -> Option<f64> {
        if index < N {
            Some(self.data[index])
        } else {
            None
        }
    }
}

// Function with const generic
pub fn create_array<const SIZE: usize>() -> [f64; SIZE] {
    [0.0; SIZE]
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check const generic struct
    let buffer = result.types.iter()
        .find(|t| t.name == "FixedBuffer")
        .expect("Should find FixedBuffer");
    
    assert!(buffer.is_generic, "FixedBuffer should be marked as generic");
    
    // Check const generic function
    let create_fn = result.functions.iter()
        .find(|f| f.name == "create_array")
        .expect("Should find create_array");
    
    assert!(create_fn.is_generic, "create_array should be marked as generic");
}