use anyhow::{Context, Result};
use blake3::Hasher;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, RwLock};
use walkdir::WalkDir;

use crate::config::Config;
use crate::graph::MemgraphClient;
use crate::parser::{RustParser, ParsedSymbols, references::resolve_all_references};
use crate::workspace::CrateMetadata;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileState {
    pub path: PathBuf,
    pub hash: String,
    pub last_modified: SystemTime,
    pub symbols_extracted: usize,
    pub functions: HashSet<String>,
    pub types: HashSet<String>,
    pub last_analyzed: SystemTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalState {
    pub files: HashMap<PathBuf, FileState>,
    pub crate_dependencies: HashMap<String, HashSet<String>>,
    pub last_full_analysis: SystemTime,
}

pub struct IncrementalUpdater {
    config: Config,
    graph: Arc<MemgraphClient>,
    parser: RustParser,
    state: Arc<RwLock<IncrementalState>>,
    file_watcher: Option<RecommendedWatcher>,
    change_sender: Option<mpsc::UnboundedSender<PathBuf>>,
}

#[derive(Debug)]
pub struct FileChange {
    pub path: PathBuf,
    pub change_type: ChangeType,
    pub timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub enum ChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: PathBuf, to: PathBuf },
}

impl IncrementalUpdater {
    pub fn new(config: Config, graph: Arc<MemgraphClient>) -> Result<Self> {
        let parser = RustParser::new()
            .context("Failed to create Rust parser")?;

        let state = Arc::new(RwLock::new(IncrementalState {
            files: HashMap::new(),
            crate_dependencies: HashMap::new(),
            last_full_analysis: SystemTime::UNIX_EPOCH,
        }));

        Ok(Self {
            config,
            graph,
            parser,
            state,
            file_watcher: None,
            change_sender: None,
        })
    }

    pub async fn load_state(&mut self, state_file: &PathBuf) -> Result<()> {
        if state_file.exists() {
            let content = std::fs::read_to_string(state_file)
                .context("Failed to read incremental state file")?;
            
            let loaded_state: IncrementalState = serde_json::from_str(&content)
                .context("Failed to deserialize incremental state")?;
            
            let mut state = self.state.write().await;
            *state = loaded_state;
            
            eprintln!("📂 Loaded incremental state with {} files", state.files.len());
        } else {
            eprintln!("📂 No existing state file, starting fresh");
        }
        
        Ok(())
    }

    pub async fn save_state(&self, state_file: &PathBuf) -> Result<()> {
        let state = self.state.read().await;
        let serialized = serde_json::to_string_pretty(&*state)
            .context("Failed to serialize incremental state")?;
        
        if let Some(parent) = state_file.parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create state directory")?;
        }
        
        std::fs::write(state_file, serialized)
            .context("Failed to write state file")?;
        
        eprintln!("💾 Saved incremental state with {} files", state.files.len());
        Ok(())
    }

    pub async fn start_watching(&mut self) -> Result<mpsc::UnboundedReceiver<PathBuf>> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.change_sender = Some(tx.clone());

        let mut watcher = notify::recommended_watcher({
            let tx = tx.clone();
            move |result: Result<Event, notify::Error>| {
                match result {
                    Ok(event) => {
                        if let Err(e) = Self::handle_file_event(&event, &tx) {
                            eprintln!("⚠️ Error handling file event: {}", e);
                        }
                    }
                    Err(e) => eprintln!("⚠️ File watcher error: {}", e),
                }
            }
        })?;

        for root in self.config.all_workspace_roots() {
            watcher.watch(root, RecursiveMode::Recursive)
                .with_context(|| format!("Failed to watch directory: {:?}", root))?;
            eprintln!("👁️ Watching directory: {:?}", root);
        }

        self.file_watcher = Some(watcher);
        Ok(rx)
    }

    fn handle_file_event(event: &Event, tx: &mpsc::UnboundedSender<PathBuf>) -> Result<()> {
        if !Self::is_rust_file(&event.paths) {
            return Ok(());
        }

        match &event.kind {
            EventKind::Create(_) | EventKind::Modify(_) => {
                for path in &event.paths {
                    let _ = tx.send(path.clone());
                }
            }
            EventKind::Remove(_) => {
                for path in &event.paths {
                    let _ = tx.send(path.clone());
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn is_rust_file(paths: &[PathBuf]) -> bool {
        paths.iter().any(|p| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "rs")
                .unwrap_or(false)
        })
    }

    pub async fn process_file_changes(&mut self, changed_files: Vec<PathBuf>) -> Result<()> {
        if changed_files.is_empty() {
            return Ok(());
        }

        let start = std::time::Instant::now();
        eprintln!("🔄 Processing {} file changes", changed_files.len());

        let mut affected_crates = HashSet::new();
        
        for file_path in &changed_files {
            if let Some(crate_name) = self.determine_crate_for_file(file_path).await? {
                affected_crates.insert(crate_name);
            }
        }

        for file_path in changed_files {
            if file_path.exists() {
                self.update_file(&file_path).await?;
            } else {
                self.remove_file(&file_path).await?;
            }
        }

        for crate_name in &affected_crates {
            self.update_crate_dependencies(crate_name).await?;
        }

        let mut state = self.state.write().await;
        state.last_full_analysis = SystemTime::now();

        let duration = start.elapsed();
        eprintln!("✅ Processed file changes in {}ms", duration.as_millis());
        Ok(())
    }

    async fn update_file(&mut self, file_path: &PathBuf) -> Result<()> {
        let file_hash = self.calculate_file_hash(file_path)?;
        let modified_time = std::fs::metadata(file_path)?.modified()?;

        let state = self.state.write().await;
        let needs_update = match state.files.get(file_path) {
            Some(existing) => existing.hash != file_hash,
            None => true,
        };

        if !needs_update {
            return Ok(());
        }
        drop(state);

        self.remove_file_symbols_from_graph(file_path).await?;

        let crate_name = self.determine_crate_for_file(file_path).await?
            .unwrap_or_else(|| "unknown".to_string());

        let mut symbols = self.parser.parse_file(file_path, &crate_name)?;
        resolve_all_references(&mut symbols)?;

        let function_ids: HashSet<String> = symbols.functions.iter()
            .map(|f| f.qualified_name.clone())
            .collect();
        let type_ids: HashSet<String> = symbols.types.iter()
            .map(|t| t.qualified_name.clone())
            .collect();

        self.graph.populate_from_symbols(&symbols).await?;

        let mut state = self.state.write().await;
        state.files.insert(file_path.clone(), FileState {
            path: file_path.clone(),
            hash: file_hash,
            last_modified: modified_time,
            symbols_extracted: symbols.functions.len() + symbols.types.len(),
            functions: function_ids,
            types: type_ids,
            last_analyzed: SystemTime::now(),
        });

        eprintln!("🔄 Updated file: {:?} ({} symbols)", file_path, symbols.functions.len() + symbols.types.len());
        Ok(())
    }

    async fn remove_file(&mut self, file_path: &PathBuf) -> Result<()> {
        self.remove_file_symbols_from_graph(file_path).await?;

        let mut state = self.state.write().await;
        if let Some(removed) = state.files.remove(file_path) {
            eprintln!("🗑️ Removed file: {:?} ({} symbols)", file_path, removed.symbols_extracted);
        }

        Ok(())
    }

    async fn remove_file_symbols_from_graph(&self, file_path: &PathBuf) -> Result<()> {
        let file_str = file_path.to_string_lossy();
        
        let delete_queries = vec![
            format!("MATCH ()-[r:CALLS]->() WHERE r.file = '{}' DELETE r", file_str),
            format!("MATCH ()-[r:USES_TYPE]->() WHERE r.file = '{}' DELETE r", file_str),
            format!("MATCH (f:Function) WHERE f.file = '{}' DETACH DELETE f", file_str),
            format!("MATCH (t:Type) WHERE t.file = '{}' DETACH DELETE t", file_str),
            format!("MATCH (m:Module) WHERE m.file = '{}' DETACH DELETE m", file_str),
        ];

        for query_str in delete_queries {
            let query = neo4rs::Query::new(query_str);
            let _ = self.graph.execute_query(query).await;
        }

        Ok(())
    }

    fn calculate_file_hash(&self, file_path: &PathBuf) -> Result<String> {
        let content = std::fs::read(file_path)
            .with_context(|| format!("Failed to read file: {:?}", file_path))?;
        
        let mut hasher = Hasher::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        
        Ok(hash.to_hex().to_string())
    }

    async fn determine_crate_for_file(&self, file_path: &PathBuf) -> Result<Option<String>> {
        let mut current_dir = file_path.parent();
        
        while let Some(dir) = current_dir {
            let cargo_toml = dir.join("Cargo.toml");
            if cargo_toml.exists() {
                if let Ok(metadata) = cargo_metadata::MetadataCommand::new()
                    .manifest_path(&cargo_toml)
                    .exec()
                {
                    if let Some(package) = metadata.packages.first() {
                        return Ok(Some(package.name.clone()));
                    }
                }
            }
            current_dir = dir.parent();
        }
        
        Ok(None)
    }

    async fn update_crate_dependencies(&self, crate_name: &str) -> Result<()> {
        let dependency_query = format!(
            "MATCH (c:Crate {{name: '{}'}})
             MATCH (c)-[:DEPENDS_ON]->(dep:Crate)
             RETURN dep.name as dependency",
            crate_name
        );

        let query = neo4rs::Query::new(dependency_query);
        let result = self.graph.execute_query(query).await?;
        let mut dependencies = HashSet::new();

        for row in result {
            if let Ok(dep_name) = row.get::<String>("dependency") {
                dependencies.insert(dep_name);
            }
        }

        let mut state = self.state.write().await;
        state.crate_dependencies.insert(crate_name.to_string(), dependencies);
        
        Ok(())
    }

    pub async fn get_file_state(&self, file_path: &PathBuf) -> Option<FileState> {
        let state = self.state.read().await;
        state.files.get(file_path).cloned()
    }

    pub async fn get_outdated_files(&self) -> Result<Vec<PathBuf>> {
        let state = self.state.read().await;
        let mut outdated = Vec::new();

        for (path, file_state) in &state.files {
            if path.exists() {
                let current_hash = self.calculate_file_hash(path)?;
                if current_hash != file_state.hash {
                    outdated.push(path.clone());
                }
            } else {
                outdated.push(path.clone());
            }
        }

        Ok(outdated)
    }

    pub async fn force_full_reanalysis(&mut self, crates: &[CrateMetadata]) -> Result<()> {
        let start = std::time::Instant::now();
        eprintln!("🔄 Starting full workspace reanalysis");

        self.graph.clear_workspace().await?;

        let mut state = self.state.write().await;
        state.files.clear();
        state.crate_dependencies.clear();
        drop(state);

        let mut all_symbols = ParsedSymbols::new();

        for crate_meta in crates {
            if crate_meta.is_workspace_member {
                let crate_symbols = self.analyze_crate_files(&crate_meta.path, &crate_meta.name).await?;
                all_symbols.merge(crate_symbols);
            }
        }

        resolve_all_references(&mut all_symbols)?;
        self.graph.populate_from_symbols(&all_symbols).await?;

        let mut state = self.state.write().await;
        state.last_full_analysis = SystemTime::now();

        let duration = start.elapsed();
        eprintln!("✅ Full reanalysis completed in {}ms", duration.as_millis());
        Ok(())
    }

    async fn analyze_crate_files(&mut self, crate_path: &PathBuf, crate_name: &str) -> Result<ParsedSymbols> {
        let mut symbols = ParsedSymbols::new();
        let src_dir = crate_path.join("src");
        
        if !src_dir.exists() {
            return Ok(symbols);
        }

        let walker = WalkDir::new(&src_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| {
                entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "rs")
                    .unwrap_or(false)
            });

        for entry in walker {
            let file_path = entry.path().to_path_buf();
            let file_symbols = self.parser.parse_file(&file_path, crate_name)?;
            symbols.merge(file_symbols);

            let file_hash = self.calculate_file_hash(&file_path)?;
            let modified_time = std::fs::metadata(&file_path)?.modified()?;

            let function_ids: HashSet<String> = symbols.functions.iter()
                .map(|f| f.qualified_name.clone())
                .collect();
            let type_ids: HashSet<String> = symbols.types.iter()
                .map(|t| t.qualified_name.clone())
                .collect();

            let mut state = self.state.write().await;
            state.files.insert(file_path.clone(), FileState {
                path: file_path.clone(),
                hash: file_hash,
                last_modified: modified_time,
                symbols_extracted: function_ids.len() + type_ids.len(),
                functions: function_ids,
                types: type_ids,
                last_analyzed: SystemTime::now(),
            });
        }

        Ok(symbols)
    }

    pub async fn get_statistics(&self) -> IncrementalStatistics {
        let state = self.state.read().await;
        
        let total_files = state.files.len();
        let total_symbols = state.files.values()
            .map(|f| f.symbols_extracted)
            .sum();
        
        let last_update = state.files.values()
            .map(|f| f.last_analyzed)
            .max()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        IncrementalStatistics {
            tracked_files: total_files,
            total_symbols,
            last_update,
            last_full_analysis: state.last_full_analysis,
            crates_tracked: state.crate_dependencies.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalStatistics {
    pub tracked_files: usize,
    pub total_symbols: usize,
    pub last_update: SystemTime,
    pub last_full_analysis: SystemTime,
    pub crates_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_hash_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        
        std::fs::write(&file_path, "fn test() {}").unwrap();
        
        let config = crate::config::Config::default();
        let graph = Arc::new(MemgraphClient::new(&config).await.unwrap());
        let updater = IncrementalUpdater::new(config, graph).unwrap();
        
        let hash1 = updater.calculate_file_hash(&file_path).unwrap();
        let hash2 = updater.calculate_file_hash(&file_path).unwrap();
        
        assert_eq!(hash1, hash2);
        
        std::fs::write(&file_path, "fn test() { println!(\"modified\"); }").unwrap();
        let hash3 = updater.calculate_file_hash(&file_path).unwrap();
        
        assert_ne!(hash1, hash3);
    }

    #[tokio::test]
    async fn test_state_serialization() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("state.json");
        
        let config = crate::config::Config::default();
        let graph = Arc::new(MemgraphClient::new(&config).await.unwrap());
        let mut updater = IncrementalUpdater::new(config, graph).unwrap();
        
        {
            let mut state = updater.state.write().await;
            state.files.insert(
                PathBuf::from("test.rs"),
                FileState {
                    path: PathBuf::from("test.rs"),
                    hash: "test-hash".to_string(),
                    last_modified: SystemTime::now(),
                    symbols_extracted: 5,
                    functions: HashSet::new(),
                    types: HashSet::new(),
                    last_analyzed: SystemTime::now(),
                }
            );
        }
        
        updater.save_state(&state_file).await.unwrap();
        assert!(state_file.exists());
        
        let config2 = crate::config::Config::default();
        let graph2 = Arc::new(MemgraphClient::new(&config2).await.unwrap());
        let mut updater2 = IncrementalUpdater::new(config2, graph2).unwrap();
        updater2.load_state(&state_file).await.unwrap();
        
        let state2 = updater2.state.read().await;
        assert_eq!(state2.files.len(), 1);
        assert!(state2.files.contains_key(&PathBuf::from("test.rs")));
    }

    #[test]
    fn test_is_rust_file() {
        assert!(IncrementalUpdater::is_rust_file(&[PathBuf::from("test.rs")]));
        assert!(IncrementalUpdater::is_rust_file(&[PathBuf::from("src/lib.rs")]));
        assert!(!IncrementalUpdater::is_rust_file(&[PathBuf::from("test.txt")]));
        assert!(!IncrementalUpdater::is_rust_file(&[PathBuf::from("Cargo.toml")]));
    }
}