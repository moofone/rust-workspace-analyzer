//! Enhanced symbol resolution using LSP semantic information
//! 
//! This module provides enhanced symbol resolution capabilities that
//! combine tree-sitter syntax analysis with LSP semantic information
//! to provide more accurate cross-crate symbol resolution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tracing::{debug, warn};

use super::{RustFunction, RustType, FunctionReference, CallType};
use crate::lsp::{
    LspManager, 
    models::{
        TypeInfo,
        ReferenceContext, Range,
        SymbolKind as LspSymbolKind
    }
};

/// Enhanced symbol resolver that combines tree-sitter and LSP data
pub struct EnhancedSymbolResolver {
    /// LSP manager for semantic queries
    lsp_manager: Option<Arc<LspManager>>,
    /// Symbol resolution cache
    resolution_cache: HashMap<String, ResolvedSymbol>,
    /// Type hierarchy cache
    type_hierarchy: HashMap<String, TypeHierarchy>,
    /// Cross-reference index
    cross_references: HashMap<String, Vec<SymbolReference>>,
    /// Confidence tracking
    confidence_scores: HashMap<String, f64>,
}

/// Resolved symbol with semantic information
#[derive(Debug, Clone)]
pub struct ResolvedSymbol {
    /// Original qualified name
    pub qualified_name: String,
    /// Resolved location
    pub location: ResolvedLocation,
    /// Symbol kind
    pub kind: ResolvedSymbolKind,
    /// Type information
    pub type_info: Option<TypeInfo>,
    /// Documentation
    pub documentation: Option<String>,
    /// Visibility
    pub visibility: SymbolVisibility,
    /// Deprecation status
    pub deprecated: bool,
    /// Resolution confidence (0.0 - 1.0)
    pub confidence: f64,
    /// Resolution method used
    pub resolution_method: ResolutionMethod,
}

/// Type hierarchy information
#[derive(Debug, Clone)]
pub struct TypeHierarchy {
    /// Type name
    pub type_name: String,
    /// Parent types (traits, base classes)
    pub parents: Vec<String>,
    /// Child types that implement/extend this
    pub children: Vec<String>,
    /// Associated functions/methods
    pub methods: Vec<String>,
    /// Associated types
    pub associated_types: Vec<String>,
}

/// Symbol reference with context
#[derive(Debug, Clone)]
pub struct SymbolReference {
    /// Reference location
    pub location: ResolvedLocation,
    /// Reference context
    pub context: ReferenceContext,
    /// Is this a cross-crate reference
    pub cross_crate: bool,
    /// Confidence in this reference
    pub confidence: f64,
}

/// Resolved symbol location
#[derive(Debug, Clone)]
pub struct ResolvedLocation {
    /// File path
    pub file_path: PathBuf,
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Symbol range
    pub range: Option<Range>,
}

/// Resolved symbol kind
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedSymbolKind {
    Function,
    Method,
    AssociatedFunction,
    Struct,
    Enum,
    Union,
    Trait,
    TypeAlias,
    Constant,
    Static,
    Module,
    Field,
    EnumVariant,
    TraitAssociatedType,
    GenericParameter,
    Macro,
    Unknown,
}

/// Symbol visibility
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolVisibility {
    Public,
    PublicCrate,
    PublicSuper,
    PublicIn(String),
    Private,
    Unknown,
}

/// Resolution method used
#[derive(Debug, Clone, PartialEq)]
pub enum ResolutionMethod {
    /// Resolved using LSP semantic information
    LspSemantic,
    /// Resolved using tree-sitter syntax
    TreeSitterSyntax,
    /// Resolved using cached information
    Cached,
    /// Resolved using pattern matching
    PatternMatching,
    /// Failed to resolve
    Failed,
}

impl EnhancedSymbolResolver {
    /// Create a new enhanced symbol resolver
    pub fn new(lsp_manager: Option<Arc<LspManager>>) -> Self {
        Self {
            lsp_manager,
            resolution_cache: HashMap::new(),
            type_hierarchy: HashMap::new(),
            cross_references: HashMap::new(),
            confidence_scores: HashMap::new(),
        }
    }

    /// Resolve a function reference using enhanced resolution
    pub async fn resolve_function_reference(
        &mut self,
        function_ref: &FunctionReference,
        context: &ResolutionContext<'_>,
    ) -> Result<ResolvedSymbol> {
        let cache_key = self.generate_cache_key(&function_ref.target_function, &function_ref.calling_function);
        
        // Check cache first
        if let Some(cached) = self.resolution_cache.get(&cache_key) {
            debug!("Using cached resolution for {}", function_ref.target_function);
            return Ok(cached.clone());
        }

        // Try LSP resolution first if available
        if let Some(lsp_manager) = &self.lsp_manager {
            if lsp_manager.is_available().await {
                match self.resolve_with_lsp(function_ref, context, lsp_manager).await {
                    Ok(resolved) => {
                        self.resolution_cache.insert(cache_key, resolved.clone());
                        return Ok(resolved);
                    }
                    Err(e) => {
                        warn!("LSP resolution failed for {}: {}", function_ref.target_function, e);
                    }
                }
            }
        }

        // Fallback to tree-sitter resolution
        let resolved = self.resolve_with_tree_sitter(function_ref, context).await?;
        self.resolution_cache.insert(cache_key, resolved.clone());
        Ok(resolved)
    }

    /// Resolve using LSP semantic information
    async fn resolve_with_lsp(
        &self,
        function_ref: &FunctionReference,
        context: &ResolutionContext<'_>,
        lsp_manager: &LspManager,
    ) -> Result<ResolvedSymbol> {
        // Get the file location for the function call
        let call_location = ResolvedLocation {
            file_path: function_ref.file_path.clone(),
            line: function_ref.line,
            column: 0, // We don't have column info from tree-sitter
            range: None,
        };

        // Try to get definition using LSP
        let definitions = lsp_manager
            .get_definition(
                &function_ref.file_path,
                (function_ref.line as u32).saturating_sub(1), // Convert to 0-based
                0,
            ).await?;

        if let Some(definition) = definitions.first() {
            let location = ResolvedLocation {
                file_path: PathBuf::from(definition.uri.trim_start_matches("file://")),
                line: definition.range.start.line as usize + 1, // Convert to 1-based
                column: definition.range.start.character as usize + 1,
                range: Some(definition.range.clone()),
            };

            // Get symbol information
            let symbols = lsp_manager
                .get_document_symbols(&location.file_path).await?;

            let matching_symbol = symbols.into_iter()
                .find(|symbol| {
                    symbol.range.start.line <= definition.range.start.line &&
                    symbol.range.end.line >= definition.range.end.line
                });

            let (kind, visibility) = if let Some(symbol) = &matching_symbol {
                (
                    self.convert_lsp_symbol_kind(&symbol.kind),
                    self.parse_visibility(&symbol.detail.as_deref().unwrap_or(""))
                )
            } else {
                (ResolvedSymbolKind::Function, SymbolVisibility::Unknown)
            };

            Ok(ResolvedSymbol {
                qualified_name: function_ref.target_function.clone(),
                location,
                kind,
                type_info: None, // Would be populated with more LSP queries
                documentation: matching_symbol.as_ref().and_then(|s| s.documentation.clone()),
                visibility,
                deprecated: matching_symbol.as_ref().map_or(false, |s| s.deprecated),
                confidence: 0.95, // High confidence for LSP resolution
                resolution_method: ResolutionMethod::LspSemantic,
            })
        } else {
            Err(anyhow::anyhow!("No definition found via LSP"))
        }
    }

    /// Resolve using tree-sitter syntax analysis
    async fn resolve_with_tree_sitter(
        &self,
        function_ref: &FunctionReference,
        context: &ResolutionContext<'_>,
    ) -> Result<ResolvedSymbol> {
        // Try to match against known functions in the workspace
        let target_name = &function_ref.target_function;
        
        // Look for exact matches in the function registry
        if let Some(function) = context.function_registry.get(target_name) {
            let location = ResolvedLocation {
                file_path: function.file_path.clone(),
                line: function.line_start,
                column: 0,
                range: None,
            };

            let kind = match function_ref.call_type {
                CallType::Method => ResolvedSymbolKind::Method,
                CallType::Macro => ResolvedSymbolKind::Macro,
                _ => ResolvedSymbolKind::Function,
            };

            let visibility = self.parse_visibility(&function.visibility);

            return Ok(ResolvedSymbol {
                qualified_name: function.qualified_name.clone(),
                location,
                kind,
                type_info: None,
                documentation: None,
                visibility,
                deprecated: false,
                confidence: 0.8, // Good confidence for exact match
                resolution_method: ResolutionMethod::TreeSitterSyntax,
            });
        }

        // Try pattern matching for partial matches
        let potential_matches = self.find_potential_matches(target_name, context);
        
        if let Some(best_match) = potential_matches.first() {
            let location = ResolvedLocation {
                file_path: best_match.file_path.clone(),
                line: best_match.line_start,
                column: 0,
                range: None,
            };

            Ok(ResolvedSymbol {
                qualified_name: best_match.qualified_name.clone(),
                location,
                kind: ResolvedSymbolKind::Function,
                type_info: None,
                documentation: None,
                visibility: self.parse_visibility(&best_match.visibility),
                deprecated: false,
                confidence: 0.6, // Lower confidence for pattern matching
                resolution_method: ResolutionMethod::PatternMatching,
            })
        } else {
            // Return unresolved symbol
            Ok(ResolvedSymbol {
                qualified_name: target_name.clone(),
                location: ResolvedLocation {
                    file_path: function_ref.file_path.clone(),
                    line: function_ref.line,
                    column: 0,
                    range: None,
                },
                kind: ResolvedSymbolKind::Unknown,
                type_info: None,
                documentation: None,
                visibility: SymbolVisibility::Unknown,
                deprecated: false,
                confidence: 0.0,
                resolution_method: ResolutionMethod::Failed,
            })
        }
    }

    /// Find potential function matches using pattern matching
    fn find_potential_matches<'a>(
        &self,
        target_name: &str,
        context: &ResolutionContext<'a>,
    ) -> Vec<&'a RustFunction> {
        let mut matches = Vec::new();
        
        // Extract the function name without module path
        let simple_name = target_name.split("::").last().unwrap_or(target_name);
        
        // Look for functions with matching simple names
        for function in context.all_functions {
            if function.name == simple_name {
                matches.push(function);
            }
        }
        
        // Sort by qualified name similarity
        matches.sort_by(|a, b| {
            let a_similarity = self.calculate_similarity(&a.qualified_name, target_name);
            let b_similarity = self.calculate_similarity(&b.qualified_name, target_name);
            b_similarity.partial_cmp(&a_similarity).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        matches
    }

    /// Calculate similarity between two strings
    fn calculate_similarity(&self, a: &str, b: &str) -> f64 {
        // Simple Levenshtein distance based similarity
        let max_len = a.len().max(b.len());
        if max_len == 0 {
            return 1.0;
        }
        
        let distance = levenshtein_distance(a, b);
        1.0 - (distance as f64 / max_len as f64)
    }

    /// Convert LSP symbol kind to our resolved symbol kind
    fn convert_lsp_symbol_kind(&self, kind: &LspSymbolKind) -> ResolvedSymbolKind {
        match kind {
            LspSymbolKind::Function => ResolvedSymbolKind::Function,
            LspSymbolKind::Method => ResolvedSymbolKind::Method,
            LspSymbolKind::Struct => ResolvedSymbolKind::Struct,
            LspSymbolKind::Enum => ResolvedSymbolKind::Enum,
            LspSymbolKind::Interface => ResolvedSymbolKind::Trait,
            LspSymbolKind::Field => ResolvedSymbolKind::Field,
            LspSymbolKind::EnumMember => ResolvedSymbolKind::EnumVariant,
            LspSymbolKind::Constant => ResolvedSymbolKind::Constant,
            LspSymbolKind::Variable => ResolvedSymbolKind::Static,
            LspSymbolKind::Module => ResolvedSymbolKind::Module,
            LspSymbolKind::TypeParameter => ResolvedSymbolKind::GenericParameter,
            _ => ResolvedSymbolKind::Unknown,
        }
    }

    /// Parse visibility from string representation
    fn parse_visibility(&self, visibility_str: &str) -> SymbolVisibility {
        match visibility_str {
            "pub" => SymbolVisibility::Public,
            "pub(crate)" => SymbolVisibility::PublicCrate,
            "pub(super)" => SymbolVisibility::PublicSuper,
            s if s.starts_with("pub(") => {
                let path = s.trim_start_matches("pub(").trim_end_matches(")");
                SymbolVisibility::PublicIn(path.to_string())
            }
            "private" | "" => SymbolVisibility::Private,
            _ => SymbolVisibility::Unknown,
        }
    }

    /// Generate cache key for resolution
    fn generate_cache_key(&self, target: &str, caller: &str) -> String {
        format!("{}|{}", target, caller)
    }

    /// Build type hierarchy for enhanced resolution
    pub async fn build_type_hierarchy(&mut self, types: &[RustType]) -> Result<()> {
        for rust_type in types {
            let hierarchy = TypeHierarchy {
                type_name: rust_type.qualified_name.clone(),
                parents: Vec::new(), // Would be populated by analyzing impl blocks
                children: Vec::new(),
                methods: Vec::new(),
                associated_types: Vec::new(),
            };
            
            self.type_hierarchy.insert(rust_type.qualified_name.clone(), hierarchy);
        }
        
        Ok(())
    }

    /// Get resolution confidence for a symbol
    pub fn get_confidence(&self, symbol_name: &str) -> f64 {
        self.confidence_scores.get(symbol_name).copied().unwrap_or(0.0)
    }

    /// Clear resolution cache
    pub fn clear_cache(&mut self) {
        self.resolution_cache.clear();
        self.confidence_scores.clear();
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        (self.resolution_cache.len(), self.type_hierarchy.len())
    }
}

/// Context for symbol resolution
pub struct ResolutionContext<'a> {
    /// Function registry for lookup
    pub function_registry: &'a HashMap<String, RustFunction>,
    /// All functions in the workspace
    pub all_functions: &'a [RustFunction],
    /// All types in the workspace
    pub all_types: &'a [RustType],
    /// Current file path
    pub current_file: &'a Path,
    /// Import statements for current file
    pub imports: &'a [String],
}

/// Simple Levenshtein distance implementation
fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();
    
    if a_len == 0 {
        return b_len;
    }
    if b_len == 0 {
        return a_len;
    }
    
    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];
    
    // Initialize first row and column
    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }
    
    // Fill the matrix
    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(
                    matrix[i - 1][j] + 1,      // deletion
                    matrix[i][j - 1] + 1,      // insertion
                ),
                matrix[i - 1][j - 1] + cost,   // substitution
            );
        }
    }
    
    matrix[a_len][b_len]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "def"), 3);
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    }

    #[test]
    fn test_visibility_parsing() {
        let resolver = EnhancedSymbolResolver::new(None);
        
        assert_eq!(resolver.parse_visibility("pub"), SymbolVisibility::Public);
        assert_eq!(resolver.parse_visibility("pub(crate)"), SymbolVisibility::PublicCrate);
        assert_eq!(resolver.parse_visibility("pub(super)"), SymbolVisibility::PublicSuper);
        assert_eq!(resolver.parse_visibility("private"), SymbolVisibility::Private);
        
        if let SymbolVisibility::PublicIn(path) = resolver.parse_visibility("pub(in crate::module)") {
            assert_eq!(path, "in crate::module");
        } else {
            panic!("Expected PublicIn variant");
        }
    }

    #[test]
    fn test_cache_key_generation() {
        let resolver = EnhancedSymbolResolver::new(None);
        let key = resolver.generate_cache_key("target::function", "caller::function");
        assert_eq!(key, "target::function|caller::function");
    }

    #[test]
    fn test_symbol_kind_conversion() {
        let resolver = EnhancedSymbolResolver::new(None);
        
        assert_eq!(
            resolver.convert_lsp_symbol_kind(&LspSymbolKind::Function),
            ResolvedSymbolKind::Function
        );
        assert_eq!(
            resolver.convert_lsp_symbol_kind(&LspSymbolKind::Struct),
            ResolvedSymbolKind::Struct
        );
        assert_eq!(
            resolver.convert_lsp_symbol_kind(&LspSymbolKind::Module),
            ResolvedSymbolKind::Module
        );
    }

    #[test]
    fn test_similarity_calculation() {
        let resolver = EnhancedSymbolResolver::new(None);
        
        assert_eq!(resolver.calculate_similarity("abc", "abc"), 1.0);
        assert_eq!(resolver.calculate_similarity("", ""), 1.0);
        
        let similarity = resolver.calculate_similarity("hello", "hallo");
        assert!(similarity > 0.5 && similarity < 1.0);
    }
}