//! Data models for LSP-enhanced analysis
//! 
//! This module defines the data structures used to represent LSP-enhanced
//! analysis results, extending the existing tree-sitter based models with
//! semantic information from rust-analyzer.

use std::collections::HashMap;
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use crate::analyzer::{RustFunction, WorkspaceSnapshot};

/// Enhanced function with LSP semantic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspEnhancedFunction {
    /// Base tree-sitter derived function data
    pub base: RustFunction,
    /// LSP semantic symbol information
    pub lsp_symbol: Option<LspSymbol>,
    /// Complete type information from LSP
    pub type_info: Option<TypeInfo>,
    /// Semantic references to this function
    pub references: Vec<LspReference>,
    /// Precise definition location from LSP
    pub definition_range: Option<Range>,
    /// Function signature with full type information
    pub signature: Option<String>,
    /// Documentation from doc comments
    pub documentation: Option<String>,
    /// Whether this function is deprecated
    pub deprecated: bool,
    /// Semantic tokens for syntax highlighting
    pub semantic_tokens: Vec<SemanticToken>,
}

/// LSP symbol information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspSymbol {
    /// Symbol kind (Function, Struct, Enum, etc.)
    pub kind: SymbolKind,
    /// Full range of the symbol
    pub range: Range,
    /// Range of just the symbol name
    pub selection_range: Range,
    /// Additional details (type signature, etc.)
    pub detail: Option<String>,
    /// Documentation string
    pub documentation: Option<String>,
    /// Whether the symbol is deprecated
    pub deprecated: bool,
    /// Tags for additional symbol information
    pub tags: Vec<SymbolTag>,
}

/// Complete type information from LSP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInfo {
    /// Type name
    pub name: String,
    /// Full type definition string
    pub definition: String,
    /// Module path where type is defined
    pub module_path: String,
    /// Generic parameters if any
    pub generic_params: Vec<String>,
    /// Implemented traits
    pub traits: Vec<String>,
    /// Fields for structs, variants for enums
    pub members: Vec<TypeMember>,
    /// Associated functions/methods
    pub methods: Vec<String>,
    /// Type aliases that reference this type
    pub aliases: Vec<String>,
}

/// Member of a type (field or enum variant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMember {
    /// Member name
    pub name: String,
    /// Member type
    pub member_type: String,
    /// Visibility (pub, pub(crate), private)
    pub visibility: String,
    /// Documentation if available
    pub documentation: Option<String>,
}

/// LSP reference to a symbol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspReference {
    /// Location of the reference
    pub location: Location,
    /// Context of the reference (read, write, declaration)
    pub context: ReferenceContext,
    /// Whether this crosses crate boundaries
    pub cross_crate: bool,
    /// The symbol being referenced
    pub symbol: String,
}

/// Result of hybrid analysis combining tree-sitter and LSP
#[derive(Debug, Clone)]
pub struct HybridAnalysisResult {
    /// Fast tree-sitter baseline analysis
    pub tree_sitter_data: WorkspaceSnapshot,
    /// LSP semantic enhancements
    pub lsp_enhancements: LspEnhancements,
    /// Strategy used to merge the data
    pub merge_strategy: MergeStrategy,
    /// When the analysis was completed
    pub analysis_timestamp: SystemTime,
    /// Whether LSP was available during analysis
    pub lsp_available: bool,
    /// Performance metrics
    pub metrics: AnalysisMetrics,
}

/// LSP enhancements to tree-sitter analysis
#[derive(Debug, Clone)]
pub struct LspEnhancements {
    /// Functions enhanced with LSP data
    pub enhanced_functions: HashMap<String, LspEnhancedFunction>,
    /// Resolved semantic references
    pub resolved_references: Vec<LspReference>,
    /// Complete type definitions
    pub type_definitions: HashMap<String, TypeInfo>,
    /// Semantic tokens for the workspace
    pub semantic_tokens: Vec<SemanticToken>,
    /// Cross-crate dependencies with semantic info
    pub semantic_dependencies: Vec<SemanticDependency>,
    /// Diagnostics from LSP
    pub diagnostics: Vec<LspDiagnostic>,
}

/// Strategy for merging tree-sitter and LSP results
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MergeStrategy {
    /// Use tree-sitter as base, enhance with LSP where available
    Progressive,
    /// Prefer LSP data when available, fallback to tree-sitter
    LspPreferred,
    /// Use only tree-sitter data
    TreeSitterOnly,
    /// Use only cached LSP data
    CachedLspOnly,
    /// Intelligent merging based on confidence scores
    Intelligent,
}

/// Performance metrics for analysis
#[derive(Debug, Clone)]
pub struct AnalysisMetrics {
    /// Time taken for tree-sitter analysis
    pub tree_sitter_duration: std::time::Duration,
    /// Time taken for LSP analysis
    pub lsp_duration: std::time::Duration,
    /// Time taken for merging results
    pub merge_duration: std::time::Duration,
    /// Number of symbols enhanced by LSP
    pub lsp_enhanced_symbols: usize,
    /// Cache hit rate for LSP queries
    pub cache_hit_rate: f64,
    /// Memory usage during analysis
    pub memory_usage: usize,
}

/// Semantic dependency with LSP information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDependency {
    /// Source symbol
    pub from_symbol: String,
    /// Target symbol
    pub to_symbol: String,
    /// Type of dependency
    pub dependency_type: DependencyType,
    /// Semantic relationship
    pub relationship: SemanticRelationship,
    /// Whether this crosses crate boundaries
    pub cross_crate: bool,
    /// Location where dependency occurs
    pub location: Location,
}

/// LSP diagnostic information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LspDiagnostic {
    /// Range where diagnostic applies
    pub range: Range,
    /// Severity level
    pub severity: DiagnosticSeverity,
    /// Diagnostic message
    pub message: String,
    /// Source of the diagnostic (rust-analyzer, etc.)
    pub source: Option<String>,
    /// Error/warning code
    pub code: Option<String>,
}

/// Position in a file
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Position {
    /// Line number (0-based)
    pub line: u32,
    /// Character offset (0-based)
    pub character: u32,
}

/// Range in a file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Range {
    /// Start position
    pub start: Position,
    /// End position
    pub end: Position,
}

/// Location in the workspace
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Location {
    /// File URI or path
    pub uri: String,
    /// Range within the file
    pub range: Range,
}

/// Symbol kind from LSP
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    File = 1,
    Module = 2,
    Namespace = 3,
    Package = 4,
    Class = 5,
    Method = 6,
    Property = 7,
    Field = 8,
    Constructor = 9,
    Enum = 10,
    Interface = 11,
    Function = 12,
    Variable = 13,
    Constant = 14,
    String = 15,
    Number = 16,
    Boolean = 17,
    Array = 18,
    Object = 19,
    Key = 20,
    Null = 21,
    EnumMember = 22,
    Struct = 23,
    Event = 24,
    Operator = 25,
    TypeParameter = 26,
}

/// Symbol tags
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolTag {
    Deprecated = 1,
}

/// Reference context
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceContext {
    /// Symbol is being read
    Read,
    /// Symbol is being written to
    Write,
    /// Symbol declaration
    Declaration,
    /// Symbol definition
    Definition,
    /// Type reference
    TypeReference,
    /// Import/use statement
    Import,
}

/// Type of semantic dependency
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    /// Function call
    FunctionCall,
    /// Type usage
    TypeUsage,
    /// Field access
    FieldAccess,
    /// Method call
    MethodCall,
    /// Trait implementation
    TraitImpl,
    /// Import/use
    Import,
    /// Inheritance/extension
    Inheritance,
}

/// Semantic relationship between symbols
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticRelationship {
    /// Direct dependency
    Direct,
    /// Indirect through another symbol
    Indirect,
    /// Bidirectional relationship
    Bidirectional,
    /// Hierarchical (parent-child)
    Hierarchical,
    /// Peer relationship
    Peer,
}

/// Diagnostic severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error = 1,
    Warning = 2,
    Information = 3,
    Hint = 4,
}

/// Semantic token for syntax highlighting
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticToken {
    /// Line number (0-based)
    pub line: u32,
    /// Character start (0-based)
    pub character: u32,
    /// Length of the token
    pub length: u32,
    /// Token type
    pub token_type: SemanticTokenType,
    /// Token modifiers
    pub modifiers: Vec<SemanticTokenModifier>,
}

/// Semantic token types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticTokenType {
    Namespace,
    Type,
    Class,
    Enum,
    Interface,
    Struct,
    TypeParameter,
    Parameter,
    Variable,
    Property,
    EnumMember,
    Event,
    Function,
    Method,
    Macro,
    Keyword,
    Modifier,
    Comment,
    String,
    Number,
    Regexp,
    Operator,
    Decorator,
}

/// Semantic token modifiers
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticTokenModifier {
    Declaration,
    Definition,
    Readonly,
    Static,
    Deprecated,
    Abstract,
    Async,
    Modification,
    Documentation,
    DefaultLibrary,
}

impl Default for MergeStrategy {
    fn default() -> Self {
        Self::Progressive
    }
}

impl Default for AnalysisMetrics {
    fn default() -> Self {
        Self {
            tree_sitter_duration: std::time::Duration::ZERO,
            lsp_duration: std::time::Duration::ZERO,
            merge_duration: std::time::Duration::ZERO,
            lsp_enhanced_symbols: 0,
            cache_hit_rate: 0.0,
            memory_usage: 0,
        }
    }
}

impl Position {
    /// Create a new position
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

impl Range {
    /// Create a new range
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
    
    /// Check if this range contains a position
    pub fn contains(&self, position: &Position) -> bool {
        position >= &self.start && position <= &self.end
    }
    
    /// Check if this range overlaps with another
    pub fn overlaps(&self, other: &Range) -> bool {
        self.start <= other.end && other.start <= self.end
    }
}

impl Location {
    /// Create a new location
    pub fn new(uri: String, range: Range) -> Self {
        Self { uri, range }
    }
    
    /// Create location from file path and range
    pub fn from_path(path: &std::path::Path, range: Range) -> Self {
        Self {
            uri: format!("file://{}", path.display()),
            range,
        }
    }
}

impl LspEnhancedFunction {
    /// Create an enhanced function from base function and LSP data
    pub fn from_base(base: RustFunction, lsp_symbol: Option<LspSymbol>) -> Self {
        Self {
            base,
            lsp_symbol,
            type_info: None,
            references: Vec::new(),
            definition_range: None,
            signature: None,
            documentation: None,
            deprecated: false,
            semantic_tokens: Vec::new(),
        }
    }
    
    /// Get the qualified name
    pub fn qualified_name(&self) -> &str {
        &self.base.qualified_name
    }
    
    /// Check if this function has LSP enhancements
    pub fn is_lsp_enhanced(&self) -> bool {
        self.lsp_symbol.is_some()
    }
    
    /// Get the best available documentation
    pub fn documentation(&self) -> Option<&str> {
        self.documentation.as_deref()
            .or_else(|| self.lsp_symbol.as_ref()?.documentation.as_deref())
    }
}

impl HybridAnalysisResult {
    /// Create a new hybrid analysis result
    pub fn new(
        tree_sitter_data: WorkspaceSnapshot,
        lsp_enhancements: LspEnhancements,
        merge_strategy: MergeStrategy,
        lsp_available: bool,
    ) -> Self {
        Self {
            tree_sitter_data,
            lsp_enhancements,
            merge_strategy,
            analysis_timestamp: SystemTime::now(),
            lsp_available,
            metrics: AnalysisMetrics::default(),
        }
    }
    
    /// Get total number of functions (tree-sitter + enhanced)
    pub fn total_functions(&self) -> usize {
        self.tree_sitter_data.functions.len()
    }
    
    /// Get number of LSP-enhanced functions
    pub fn enhanced_functions_count(&self) -> usize {
        self.lsp_enhancements.enhanced_functions.len()
    }
    
    /// Get enhancement percentage
    pub fn enhancement_percentage(&self) -> f64 {
        if self.total_functions() == 0 {
            0.0
        } else {
            (self.enhanced_functions_count() as f64 / self.total_functions() as f64) * 100.0
        }
    }
    
    /// Check if a function is LSP-enhanced
    pub fn is_function_enhanced(&self, qualified_name: &str) -> bool {
        self.lsp_enhancements.enhanced_functions.contains_key(qualified_name)
    }
    
    /// Get enhanced function by qualified name
    pub fn get_enhanced_function(&self, qualified_name: &str) -> Option<&LspEnhancedFunction> {
        self.lsp_enhancements.enhanced_functions.get(qualified_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(5, 10);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.character, 10);
    }

    #[test]
    fn test_range_contains() {
        let range = Range::new(Position::new(1, 0), Position::new(3, 10));
        assert!(range.contains(&Position::new(2, 5)));
        assert!(!range.contains(&Position::new(0, 5)));
        assert!(!range.contains(&Position::new(4, 5)));
    }

    #[test]
    fn test_range_overlaps() {
        let range1 = Range::new(Position::new(1, 0), Position::new(3, 10));
        let range2 = Range::new(Position::new(2, 5), Position::new(4, 0));
        let range3 = Range::new(Position::new(5, 0), Position::new(6, 0));
        
        assert!(range1.overlaps(&range2));
        assert!(!range1.overlaps(&range3));
    }

    #[test]
    fn test_location_from_path() {
        let path = std::path::Path::new("/test/file.rs");
        let range = Range::new(Position::new(1, 0), Position::new(1, 10));
        let location = Location::from_path(path, range.clone());
        
        assert_eq!(location.uri, "file:///test/file.rs");
        assert_eq!(location.range, range);
    }

    #[test]
    fn test_merge_strategy_default() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Progressive);
    }

    #[test]
    fn test_hybrid_analysis_result() {
        let snapshot = WorkspaceSnapshot {
            functions: vec![],
            types: vec![],
            dependencies: vec![],
            function_references: vec![],
            function_registry: crate::analyzer::FunctionRegistry {
                functions_by_name: HashMap::new(),
                functions_by_qualified: HashMap::new(),
                public_functions: std::collections::HashSet::new(),
            },
            timestamp: SystemTime::now(),
        };
        
        let enhancements = LspEnhancements {
            enhanced_functions: HashMap::new(),
            resolved_references: vec![],
            type_definitions: HashMap::new(),
            semantic_tokens: vec![],
            semantic_dependencies: vec![],
            diagnostics: vec![],
        };
        
        let result = HybridAnalysisResult::new(
            snapshot,
            enhancements,
            MergeStrategy::Progressive,
            true,
        );
        
        assert_eq!(result.total_functions(), 0);
        assert_eq!(result.enhanced_functions_count(), 0);
        assert_eq!(result.enhancement_percentage(), 0.0);
        assert!(result.lsp_available);
    }
}