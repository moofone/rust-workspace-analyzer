// Test fixtures for trait implementations

pub const SIMPLE_TRAIT_IMPL: &str = r#"
use std::fmt::{Display, Formatter, Result};

struct MyType {
    value: i32,
}

impl Display for MyType {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "MyType({})", self.value)
    }
}
"#;

pub const GENERIC_TRAIT_IMPL: &str = r#"
use std::fmt::{Debug, Display, Formatter, Result};

struct Container<T> {
    item: T,
}

impl<T: Debug> Display for Container<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Container({:?})", self.item)
    }
}
"#;

pub const INHERENT_IMPL: &str = r#"
struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new(value: i32) -> Self {
        Self { value }
    }
    
    pub fn add(&mut self, other: i32) -> i32 {
        self.value += other;
        self.value
    }
    
    async fn async_calculate(&self) -> i32 {
        self.value * 2
    }
}
"#;

pub const MULTIPLE_TRAIT_IMPLS: &str = r#"
use std::fmt::{Debug, Display, Formatter, Result};

#[derive(Clone)]
struct Point {
    x: f64,
    y: f64,
}

impl Debug for Point {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "Point {{ x: {}, y: {} }}", self.x, self.y)
    }
}

impl Display for Point {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    
    pub fn distance(&self, other: &Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}
"#;

pub const ASYNC_TRAIT_IMPL: &str = r#"
use async_trait::async_trait;

#[async_trait]
trait AsyncProcessor {
    async fn process(&self, data: &str) -> String;
}

struct SimpleProcessor;

#[async_trait]
impl AsyncProcessor for SimpleProcessor {
    async fn process(&self, data: &str) -> String {
        format!("Processed: {}", data)
    }
}
"#;

pub const TRAIT_WITH_DEFAULT_METHODS: &str = r#"
trait Configurable {
    fn get_config_value(&self, key: &str) -> Option<String>;
    
    fn is_enabled(&self, feature: &str) -> bool {
        self.get_config_value(feature)
            .map(|v| v == "true")
            .unwrap_or(false)
    }
    
    fn get_timeout(&self) -> u64 {
        self.get_config_value("timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(5000)
    }
}

struct AppConfig {
    values: std::collections::HashMap<String, String>,
}

impl Configurable for AppConfig {
    fn get_config_value(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
    
    fn get_timeout(&self) -> u64 {
        // Override default implementation
        self.get_config_value("custom_timeout")
            .and_then(|s| s.parse().ok())
            .unwrap_or(10000)
    }
}
"#;