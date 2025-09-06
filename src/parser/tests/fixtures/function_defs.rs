// Test fixtures for function definitions

pub const SIMPLE_FUNCTION: &str = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;

pub const ASYNC_FUNCTION: &str = r#"
async fn fetch_data(url: &str) -> Result<String, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let text = response.text().await?;
    Ok(text)
}
"#;

pub const GENERIC_FUNCTION: &str = r#"
fn process_items<T, U, F>(items: Vec<T>, transform: F) -> Vec<U>
where
    T: Clone,
    F: Fn(T) -> U,
{
    items.into_iter().map(transform).collect()
}
"#;

pub const UNSAFE_FUNCTION: &str = r#"
unsafe fn raw_pointer_access(ptr: *const i32) -> i32 {
    *ptr
}
"#;

pub const CONST_FUNCTION: &str = r#"
const fn calculate_size(width: usize, height: usize) -> usize {
    width * height
}
"#;

pub const FUNCTION_WITH_LIFETIMES: &str = r#"
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}
"#;

pub const CLOSURE_EXAMPLES: &str = r#"
fn closure_examples() {
    let add_one = |x: i32| x + 1;
    let multiply = |x: i32, y: i32| -> i32 { x * y };
    
    let capture_environment = || {
        let local_var = 42;
        move |x: i32| x + local_var
    };
}
"#;

pub const FUNCTION_WITH_ATTRIBUTES: &str = r#"
#[cfg(test)]
#[tokio::test]
async fn test_async_function() {
    let result = fetch_data("http://example.com").await;
    assert!(result.is_ok());
}

#[inline]
#[must_use]
fn important_calculation(x: i32) -> i32 {
    x * 2 + 1
}

#[deprecated(since = "1.0.0", note = "use new_function instead")]
fn old_function() {
    // deprecated implementation
}
"#;

pub const METHOD_DEFINITIONS: &str = r#"
struct Calculator {
    value: i32,
}

impl Calculator {
    pub fn new(initial: i32) -> Self {
        Self { value: initial }
    }
    
    pub fn add(&mut self, other: i32) -> &mut Self {
        self.value += other;
        self
    }
    
    pub fn get_value(&self) -> i32 {
        self.value
    }
    
    fn internal_helper(&self) -> i32 {
        self.value * 2
    }
    
    pub async fn async_process(&mut self) -> i32 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        self.value += 1;
        self.value
    }
}
"#;

pub const NESTED_FUNCTIONS: &str = r#"
fn outer_function(x: i32) -> impl Fn(i32) -> i32 {
    fn inner_function(y: i32) -> i32 {
        y * 2
    }
    
    move |z| inner_function(z + x)
}

mod nested_module {
    pub fn module_function() {
        fn local_helper() {
            println!("Helper function inside module function");
        }
        local_helper();
    }
}
"#;