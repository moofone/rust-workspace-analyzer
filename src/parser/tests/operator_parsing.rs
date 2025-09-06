use crate::parser::RustParser;
use std::path::Path;

/// Test operator overloading parsing from trading-ta/Pf64 patterns
#[test]
pub fn test_operator_parsing() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    // Test case inspired by Pf64 operator implementations
    let source = r#"
use std::ops::{Add, Sub, Mul, Div, Neg};
use std::cmp::{PartialEq, PartialOrd, Ordering};

#[derive(Debug, Clone, Copy)]
pub struct Pf64(pub f64);

// Arithmetic operators for Pf64
impl Add for Pf64 {
    type Output = Self;
    
    fn add(self, other: Self) -> Self::Output {
        Pf64(self.0 + other.0)
    }
}

impl Sub for Pf64 {
    type Output = Self;
    
    fn sub(self, other: Self) -> Self::Output {
        Pf64(self.0 - other.0)
    }
}

impl Mul for Pf64 {
    type Output = Self;
    
    fn mul(self, other: Self) -> Self::Output {
        Pf64(self.0 * other.0)
    }
}

impl Div for Pf64 {
    type Output = Self;
    
    fn div(self, other: Self) -> Self::Output {
        Pf64(self.0 / other.0)
    }
}

// Operators for f64 with Pf64 (critical pattern)
impl Add<Pf64> for f64 {
    type Output = Pf64;
    
    fn add(self, other: Pf64) -> Pf64 {
        Pf64(self + other.0)
    }
}

impl Sub<Pf64> for f64 {
    type Output = Pf64;
    
    fn sub(self, other: Pf64) -> Pf64 {
        Pf64(self - other.0)
    }
}

impl Mul<Pf64> for f64 {
    type Output = Pf64;
    
    fn mul(self, other: Pf64) -> Pf64 {
        Pf64(self * other.0)
    }
}

impl Div<Pf64> for f64 {
    type Output = Pf64;
    
    fn div(self, other: Pf64) -> Pf64 {
        Pf64(self / other.0)
    }
}

// Unary operator
impl Neg for Pf64 {
    type Output = Self;
    
    fn neg(self) -> Self::Output {
        Pf64(-self.0)
    }
}

// Comparison operators
impl PartialEq for Pf64 {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialOrd for Pf64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Debug: see what functions were found
    println!("Found {} functions total", result.functions.len());
    for func in &result.functions {
        println!("  Function: {} (is_trait_impl: {})", func.name, func.is_trait_impl);
    }
    
    // All operator implementations should be marked as trait implementations
    let operators = ["add", "sub", "mul", "div", "neg", "eq", "partial_cmp"];
    
    for op_name in &operators {
        let implementations: Vec<_> = result.functions.iter()
            .filter(|f| f.name == *op_name)
            .collect();
        
        assert!(!implementations.is_empty(), 
                "Should find {} operator implementation", op_name);
        
        for impl_fn in implementations {
            assert!(impl_fn.is_trait_impl, 
                    "{} should be marked as trait implementation", op_name);
        }
    }
    
    // Specifically check for f64 operator implementations (critical for not showing as unused)
    println!("\nLooking for f64 implementations:");
    for func in &result.functions {
        if func.name == "add" {
            println!("  add function with qualified_name: {}", func.qualified_name);
        }
    }
    
    let f64_add = result.functions.iter()
        .find(|f| f.name == "add" && f.qualified_name.contains("f64"))
        .expect("Should find add implementation for f64");
    
    assert!(f64_add.is_trait_impl, "f64::add should be marked as trait implementation");
    
    let f64_sub = result.functions.iter()
        .find(|f| f.name == "sub" && f.qualified_name.contains("f64"))
        .expect("Should find sub implementation for f64");
    
    assert!(f64_sub.is_trait_impl, "f64::sub should be marked as trait implementation");
}

/// Test assignment operators
#[test]
fn test_assignment_operators() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use std::ops::{AddAssign, SubAssign, MulAssign, DivAssign};

pub struct Counter(pub i64);

impl AddAssign for Counter {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign<i64> for Counter {
    fn sub_assign(&mut self, value: i64) {
        self.0 -= value;
    }
}

impl MulAssign<f64> for Counter {
    fn mul_assign(&mut self, factor: f64) {
        self.0 = (self.0 as f64 * factor) as i64;
    }
}

impl DivAssign for Counter {
    fn div_assign(&mut self, divisor: Self) {
        self.0 /= divisor.0;
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check assignment operators
    let assign_ops = ["add_assign", "sub_assign", "mul_assign", "div_assign"];
    
    for op_name in &assign_ops {
        let op_fn = result.functions.iter()
            .find(|f| f.name == *op_name)
            .expect(&format!("Should find {} operator", op_name));
        
        assert!(op_fn.is_trait_impl, "{} should be trait implementation", op_name);
        
        // Debug: print parameters
        println!("{} parameters:", op_name);
        for param in &op_fn.parameters {
            println!("  - {} (type: {}, is_self: {}, is_mutable: {})", 
                     param.name, param.param_type, param.is_self, param.is_mutable);
        }
        
        // Assignment operators take &mut self
        assert!(op_fn.parameters.iter().any(|p| p.is_self && p.is_mutable),
                "{} should have &mut self parameter", op_name);
    }
}

/// Test index and deref operators
#[test]
fn test_index_deref_operators() {
    let mut parser = RustParser::new().expect("Failed to create parser");
    
    let source = r#"
use std::ops::{Index, IndexMut, Deref, DerefMut};

pub struct DataArray {
    data: Vec<f64>,
}

impl Index<usize> for DataArray {
    type Output = f64;
    
    fn index(&self, idx: usize) -> &Self::Output {
        &self.data[idx]
    }
}

impl IndexMut<usize> for DataArray {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.data[idx]
    }
}

impl Deref for DataArray {
    type Target = [f64];
    
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for DataArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
"#;

    let result = parser.parse_source(
        source,
        Path::new("test.rs"),
        "test_crate"
    ).unwrap();

    // Check index and deref operators
    let special_ops = ["index", "index_mut", "deref", "deref_mut"];
    
    for op_name in &special_ops {
        let op_fn = result.functions.iter()
            .find(|f| f.name == *op_name)
            .expect(&format!("Should find {} operator", op_name));
        
        assert!(op_fn.is_trait_impl, "{} should be trait implementation", op_name);
    }
}