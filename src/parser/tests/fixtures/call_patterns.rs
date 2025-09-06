// Test fixtures for call detection patterns

pub const SIMPLE_CALLS: &str = r#"
fn main() {
    println!("Hello world");
    let result = add(5, 3);
    process_data();
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn process_data() {
    // some processing
}
"#;

pub const METHOD_CALLS: &str = r#"
struct Calculator {
    value: i32,
}

impl Calculator {
    fn new() -> Self {
        Self { value: 0 }
    }
    
    fn add(&mut self, x: i32) -> i32 {
        self.value += x;
        self.value
    }
    
    fn get_value(&self) -> i32 {
        self.value
    }
}

fn test_methods() {
    let mut calc = Calculator::new();
    calc.add(10);
    let result = calc.get_value();
}
"#;

pub const ASSOCIATED_FUNCTION_CALLS: &str = r#"
use std::collections::HashMap;

fn test_associated_calls() {
    let map = HashMap::new();
    let vec = Vec::with_capacity(10);
    let string = String::from("hello");
    let result = i32::from_str("42");
}
"#;

pub const UFCS_CALLS: &str = r#"
use std::fmt::Display;

fn test_ufcs() {
    let s = String::from("hello");
    
    // Universal Function Call Syntax
    let len1 = str::len(&s);
    let len2 = <str>::len(&s);
    let display = <String as Display>::fmt(&s, &mut formatter);
}
"#;

pub const CHAINED_CALLS: &str = r#"
fn test_chaining() {
    let result = vec![1, 2, 3, 4, 5]
        .iter()
        .filter(|&x| *x > 2)
        .map(|x| x * 2)
        .collect::<Vec<_>>();
        
    let processed = "hello world"
        .split_whitespace()
        .map(str::to_uppercase)
        .collect::<Vec<String>>()
        .join(" ");
}
"#;

pub const MACRO_CALLS: &str = r#"
fn test_macros() {
    println!("Debug: {:?}", value);
    vec![1, 2, 3];
    format!("Hello, {}!", name);
    dbg!(some_expression);
    
    // Custom macros
    my_macro!(arg1, arg2);
    complex_macro! {
        field: value,
        other: "string"
    };
}
"#;

pub const ASYNC_CALLS: &str = r#"
async fn async_function() -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::get("https://api.example.com").await?;
    let text = response.text().await?;
    
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    
    Ok(text)
}

async fn caller() {
    let result = async_function().await;
    match result {
        Ok(data) => println!("Got: {}", data),
        Err(e) => eprintln!("Error: {}", e),
    }
}
"#;

pub const GENERIC_CALLS: &str = r#"
fn generic_calls() {
    let vec: Vec<i32> = Vec::new();
    let result = Some(42).map(|x| x.to_string());
    
    let collected: HashMap<String, i32> = items
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        
    let parsed = "42".parse::<i32>().unwrap();
}
"#;

pub const NESTED_CALLS: &str = r#"
fn nested_calls() {
    let result = outer_function(
        inner_function(
            deepest_function(42)
        )
    );
    
    let complex = calculate(
        get_base_value(),
        transform(
            preprocess(
                load_data("config.json")
            )
        )
    );
}
"#;