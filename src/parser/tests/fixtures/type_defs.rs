// Test fixtures for type definitions (structs, enums, etc.)

pub const SIMPLE_STRUCT: &str = r#"
struct Point {
    x: f64,
    y: f64,
}
"#;

pub const STRUCT_WITH_VISIBILITY: &str = r#"
pub struct User {
    pub id: u64,
    pub name: String,
    email: String,
    pub(crate) internal_id: u32,
}
"#;

pub const GENERIC_STRUCT: &str = r#"
pub struct Container<T, U = String> 
where 
    T: Clone + Send,
    U: Display,
{
    pub primary: T,
    secondary: U,
    metadata: HashMap<String, String>,
}
"#;

pub const TUPLE_STRUCT: &str = r#"
pub struct Color(pub u8, pub u8, pub u8, pub u8);
pub struct Wrapper<T>(T);
"#;

pub const UNIT_STRUCT: &str = r#"
pub struct Marker;
struct EmptyState;
"#;

pub const SIMPLE_ENUM: &str = r#"
pub enum Status {
    Active,
    Inactive,
    Pending,
}
"#;

pub const ENUM_WITH_DATA: &str = r#"
pub enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(u8, u8, u8),
    Complex {
        id: u64,
        data: Vec<u8>,
        metadata: Option<HashMap<String, String>>,
    },
}
"#;

pub const GENERIC_ENUM: &str = r#"
pub enum Result<T, E> {
    Ok(T),
    Err(E),
}

pub enum Option<T> {
    Some(T),
    None,
}
"#;

pub const ENUM_WITH_ATTRIBUTES: &str = r#"
#[derive(Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiResponse {
    #[serde(rename = "success")]
    Success {
        data: String,
        timestamp: u64,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        code: i32,
    },
    #[deprecated]
    LegacyResponse,
}
"#;

pub const TRAIT_DEFINITIONS: &str = r#"
pub trait Drawable {
    fn draw(&self);
    
    fn area(&self) -> f64 {
        0.0
    }
}

pub trait Iterator {
    type Item;
    
    fn next(&mut self) -> Option<Self::Item>;
    
    fn collect<B: FromIterator<Self::Item>>(self) -> B
    where
        Self: Sized,
    {
        FromIterator::from_iter(self)
    }
}

pub trait AsyncProcessor<T> {
    type Output;
    type Error;
    
    async fn process(&mut self, input: T) -> Result<Self::Output, Self::Error>;
}
"#;

pub const TYPE_ALIASES: &str = r#"
pub type UserId = u64;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub type ComplexType<T, U> = HashMap<T, Vec<Option<U>>>;

type InternalResult = Result<String>;
"#;

pub const STRUCT_WITH_DOC_COMMENTS: &str = r#"
/// Represents a 2D point in space
/// 
/// This struct provides basic functionality for working with
/// coordinates in a 2D plane.
#[derive(Debug, Clone, Copy)]
pub struct Point {
    /// The x coordinate
    pub x: f64,
    /// The y coordinate  
    pub y: f64,
}
"#;