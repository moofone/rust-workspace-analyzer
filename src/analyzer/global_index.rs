use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{BufReader, BufWriter};

/// Information about a function in a crate for cross-crate resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateFunctionInfo {
    pub name: String,
    pub crate_name: String,
    pub module_path: Vec<String>,
    pub signature: String,
    pub visibility: Visibility,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub is_extern: bool,
    pub associated_type: Option<String>,
    pub trait_impl: Option<String>,
    pub file_path: PathBuf,
    pub line_number: Option<u32>,
}

/// Information about a type in a crate for cross-crate resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateTypeInfo {
    pub name: String,
    pub crate_name: String,
    pub module_path: Vec<String>,
    pub type_kind: TypeKind,
    pub visibility: Visibility,
    pub methods: Vec<String>,
    pub associated_functions: Vec<String>,
    pub trait_impls: Vec<String>,
    pub generic_params: Vec<String>,
    pub file_path: PathBuf,
    pub line_number: Option<u32>,
}

/// Information about a trait in a crate for cross-crate resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateTraitInfo {
    pub name: String,
    pub crate_name: String,
    pub module_path: Vec<String>,
    pub visibility: Visibility,
    pub methods: Vec<TraitMethodInfo>,
    pub associated_types: Vec<String>,
    pub super_traits: Vec<String>,
    pub generic_params: Vec<String>,
    pub file_path: PathBuf,
    pub line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitMethodInfo {
    pub name: String,
    pub signature: String,
    pub is_async: bool,
    pub is_unsafe: bool,
    pub has_default_impl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    Struct,
    Enum,
    Union,
    Alias,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Crate,
    SuperScope,
    Private,
}

/// Global symbol index for cross-crate resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalSymbolIndex {
    pub functions: HashMap<String, Vec<CrateFunctionInfo>>,
    pub types: HashMap<String, Vec<CrateTypeInfo>>,
    pub traits: HashMap<String, Vec<CrateTraitInfo>>,
    pub crate_exports: HashMap<String, CrateExports>,
    pub last_updated: std::time::SystemTime,
    pub workspace_root: PathBuf,
}

/// Exports from a specific crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateExports {
    pub crate_name: String,
    pub public_functions: Vec<String>,
    pub public_types: Vec<String>,
    pub public_traits: Vec<String>,
    pub re_exports: HashMap<String, String>, // alias -> original
    pub glob_exports: Vec<String>, // module paths with glob exports
}

impl GlobalSymbolIndex {
    pub fn new(workspace_root: PathBuf) -> Self {
        Self {
            functions: HashMap::new(),
            types: HashMap::new(),
            traits: HashMap::new(),
            crate_exports: HashMap::new(),
            last_updated: std::time::SystemTime::now(),
            workspace_root,
        }
    }

    /// Add function information to the index
    pub fn add_function(&mut self, func_info: CrateFunctionInfo) {
        self.functions
            .entry(func_info.name.clone())
            .or_insert_with(Vec::new)
            .push(func_info);
    }

    /// Add type information to the index
    pub fn add_type(&mut self, type_info: CrateTypeInfo) {
        self.types
            .entry(type_info.name.clone())
            .or_insert_with(Vec::new)
            .push(type_info);
    }

    /// Add trait information to the index
    pub fn add_trait(&mut self, trait_info: CrateTraitInfo) {
        self.traits
            .entry(trait_info.name.clone())
            .or_insert_with(Vec::new)
            .push(trait_info);
    }

    /// Find function by name across all crates
    pub fn find_function(&self, name: &str) -> Option<&Vec<CrateFunctionInfo>> {
        self.functions.get(name)
    }

    /// Find type by name across all crates
    pub fn find_type(&self, name: &str) -> Option<&Vec<CrateTypeInfo>> {
        self.types.get(name)
    }

    /// Find trait by name across all crates
    pub fn find_trait(&self, name: &str) -> Option<&Vec<CrateTraitInfo>> {
        self.traits.get(name)
    }

    /// Find function in specific crate
    pub fn find_function_in_crate(&self, name: &str, crate_name: &str) -> Option<&CrateFunctionInfo> {
        self.functions.get(name)?.iter()
            .find(|f| f.crate_name == crate_name)
    }

    /// Find type in specific crate
    pub fn find_type_in_crate(&self, name: &str, crate_name: &str) -> Option<&CrateTypeInfo> {
        self.types.get(name)?.iter()
            .find(|t| t.crate_name == crate_name)
    }

    /// Find trait in specific crate
    pub fn find_trait_in_crate(&self, name: &str, crate_name: &str) -> Option<&CrateTraitInfo> {
        self.traits.get(name)?.iter()
            .find(|t| t.crate_name == crate_name)
    }

    /// Resolve Type::method call across crates
    pub fn resolve_associated_function(&self, type_name: &str, method_name: &str) -> Vec<&CrateFunctionInfo> {
        let mut results = Vec::new();
        
        // Look for associated functions on types
        if let Some(type_infos) = self.types.get(type_name) {
            for type_info in type_infos {
                if type_info.associated_functions.contains(&method_name.to_string()) {
                    // Find the actual function
                    if let Some(func_infos) = self.functions.get(method_name) {
                        for func_info in func_infos {
                            if func_info.associated_type.as_ref() == Some(&type_info.name) &&
                               func_info.crate_name == type_info.crate_name {
                                results.push(func_info);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Resolve trait method calls through dynamic dispatch
    pub fn resolve_trait_method(&self, trait_name: &str, method_name: &str) -> Vec<&CrateFunctionInfo> {
        let mut results = Vec::new();
        
        if let Some(trait_infos) = self.traits.get(trait_name) {
            for trait_info in trait_infos {
                if trait_info.methods.iter().any(|m| m.name == method_name) {
                    // Find implementations of this trait method
                    if let Some(func_infos) = self.functions.get(method_name) {
                        for func_info in func_infos {
                            if func_info.trait_impl.as_ref() == Some(&trait_info.name) {
                                results.push(func_info);
                            }
                        }
                    }
                }
            }
        }

        results
    }

    /// Get crate exports for import resolution
    pub fn get_crate_exports(&self, crate_name: &str) -> Option<&CrateExports> {
        self.crate_exports.get(crate_name)
    }

    /// Add crate exports information
    pub fn add_crate_exports(&mut self, exports: CrateExports) {
        self.crate_exports.insert(exports.crate_name.clone(), exports);
    }

    /// Clear all data (for rebuilding)
    pub fn clear(&mut self) {
        self.functions.clear();
        self.types.clear();
        self.traits.clear();
        self.crate_exports.clear();
        self.last_updated = std::time::SystemTime::now();
    }

    /// Get statistics about the index
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            total_functions: self.functions.values().map(|v| v.len()).sum(),
            total_types: self.types.values().map(|v| v.len()).sum(),
            total_traits: self.traits.values().map(|v| v.len()).sum(),
            total_crates: self.crate_exports.len(),
            unique_function_names: self.functions.len(),
            unique_type_names: self.types.len(),
            unique_trait_names: self.traits.len(),
        }
    }

    /// Save the index to disk using bincode with compression
    pub fn save_to_disk<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        
        // Use bincode for efficient binary serialization
        bincode::serialize_into(writer, self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize global symbol index: {}", e))?;
        
        Ok(())
    }

    /// Load the index from disk using bincode
    pub fn load_from_disk<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        
        let index: GlobalSymbolIndex = bincode::deserialize_from(reader)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize global symbol index: {}", e))?;
        
        Ok(index)
    }

    /// Save with compression using flate2
    pub fn save_compressed<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        
        let file = File::create(path)?;
        let encoder = GzEncoder::new(BufWriter::new(file), Compression::default());
        
        bincode::serialize_into(encoder, self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize compressed global symbol index: {}", e))?;
        
        Ok(())
    }

    /// Load with decompression using flate2
    pub fn load_compressed<P: AsRef<Path>>(path: P) -> Result<Self> {
        use flate2::read::GzDecoder;
        
        let file = File::open(path)?;
        let decoder = GzDecoder::new(BufReader::new(file));
        
        let index: GlobalSymbolIndex = bincode::deserialize_from(decoder)
            .map_err(|e| anyhow::anyhow!("Failed to deserialize compressed global symbol index: {}", e))?;
        
        Ok(index)
    }

    /// Check if the index file exists and is newer than the workspace
    pub fn is_cache_valid<P: AsRef<Path>>(&self, cache_path: P) -> Result<bool> {
        let cache_path = cache_path.as_ref();
        
        if !cache_path.exists() {
            return Ok(false);
        }

        let cache_modified = cache_path.metadata()?.modified()?;
        
        // Check if any Cargo.toml or Cargo.lock files are newer than the cache
        let workspace_files = [
            self.workspace_root.join("Cargo.toml"),
            self.workspace_root.join("Cargo.lock"),
        ];

        for file_path in &workspace_files {
            if file_path.exists() {
                let file_modified = file_path.metadata()?.modified()?;
                if file_modified > cache_modified {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Get the default cache path for the workspace
    pub fn default_cache_path(&self) -> PathBuf {
        self.workspace_root.join("target").join(".workspace-analyzer-cache.bin.gz")
    }

    /// Try to load from cache, or return None if cache is invalid
    pub fn try_load_from_cache(&self) -> Result<Option<GlobalSymbolIndex>> {
        let cache_path = self.default_cache_path();
        
        if !self.is_cache_valid(&cache_path)? {
            return Ok(None);
        }

        match Self::load_compressed(&cache_path) {
            Ok(index) => Ok(Some(index)),
            Err(_) => {
                // Cache file might be corrupted, ignore and rebuild
                Ok(None)
            }
        }
    }

    /// Save to default cache location
    pub fn save_to_cache(&self) -> Result<()> {
        let cache_path = self.default_cache_path();
        
        // Ensure target directory exists
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        self.save_compressed(&cache_path)
    }
}

#[derive(Debug)]
pub struct IndexStats {
    pub total_functions: usize,
    pub total_types: usize,
    pub total_traits: usize,
    pub total_crates: usize,
    pub unique_function_names: usize,
    pub unique_type_names: usize,
    pub unique_trait_names: usize,
}

impl std::fmt::Display for IndexStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, 
            "GlobalSymbolIndex Stats:\n\
             - Functions: {} ({} unique names)\n\
             - Types: {} ({} unique names)\n\
             - Traits: {} ({} unique names)\n\
             - Crates: {}",
            self.total_functions, self.unique_function_names,
            self.total_types, self.unique_type_names,
            self.total_traits, self.unique_trait_names,
            self.total_crates
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_symbol_index_creation() {
        let index = GlobalSymbolIndex::new("/test".into());
        assert_eq!(index.functions.len(), 0);
        assert_eq!(index.types.len(), 0);
        assert_eq!(index.traits.len(), 0);
    }

    #[test]
    fn test_add_and_find_function() {
        let mut index = GlobalSymbolIndex::new("/test".into());
        
        let func_info = CrateFunctionInfo {
            name: "test_func".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: vec!["test".to_string()],
            signature: "fn test_func()".to_string(),
            visibility: Visibility::Public,
            is_async: false,
            is_unsafe: false,
            is_extern: false,
            associated_type: None,
            trait_impl: None,
            file_path: "/test/lib.rs".into(),
            line_number: Some(10),
        };

        index.add_function(func_info.clone());
        
        let found = index.find_function("test_func");
        assert!(found.is_some());
        assert_eq!(found.unwrap().len(), 1);
        assert_eq!(found.unwrap()[0].name, "test_func");
    }

    #[test]
    fn test_resolve_associated_function() {
        let mut index = GlobalSymbolIndex::new("/test".into());
        
        // Add a type with an associated function
        let type_info = CrateTypeInfo {
            name: "TestType".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: vec![],
            type_kind: TypeKind::Struct,
            visibility: Visibility::Public,
            methods: vec![],
            associated_functions: vec!["new".to_string()],
            trait_impls: vec![],
            generic_params: vec![],
            file_path: "/test/lib.rs".into(),
            line_number: Some(5),
        };
        
        let func_info = CrateFunctionInfo {
            name: "new".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: vec![],
            signature: "fn new() -> Self".to_string(),
            visibility: Visibility::Public,
            is_async: false,
            is_unsafe: false,
            is_extern: false,
            associated_type: Some("TestType".to_string()),
            trait_impl: None,
            file_path: "/test/lib.rs".into(),
            line_number: Some(10),
        };

        index.add_type(type_info);
        index.add_function(func_info);
        
        let results = index.resolve_associated_function("TestType", "new");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "new");
    }
}