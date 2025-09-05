use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateMetadata {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub layer: Option<String>,
    pub depth: usize,
    pub dependencies: Vec<String>,
    pub is_workspace_member: bool,
    pub is_external: bool,
}

#[derive(Debug)]
pub struct WorkspaceDiscovery {
    config: Config,
    discovered: HashMap<String, CrateMetadata>,
}

const MAX_DEPTH: usize = 3;

impl WorkspaceDiscovery {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            discovered: HashMap::new(),
        }
    }

    pub async fn discover_crates(&mut self) -> Result<Vec<CrateMetadata>> {
        let mut to_scan = VecDeque::new();
        let mut visited = HashSet::new();

        for root in self.config.all_workspace_roots() {
            self.scan_workspace_root(root, &mut to_scan, &mut visited)
                .with_context(|| format!("Failed to scan workspace root: {:?}", root))?;
        }

        while let Some((package_id, depth)) = to_scan.pop_front() {
            if visited.contains(&package_id) || depth > MAX_DEPTH {
                continue;
            }
            
            if let Some(crate_meta) = self.resolve_crate_metadata(&package_id, depth)? {
                visited.insert(package_id.clone());
                
                if self.config.analysis.recursive_scan && depth < MAX_DEPTH {
                    for dep in &crate_meta.dependencies {
                        if !visited.contains(dep) {
                            to_scan.push_back((dep.clone(), depth + 1));
                        }
                    }
                }
                
                self.discovered.insert(crate_meta.name.clone(), crate_meta);
            }
        }

        Ok(self.discovered.values().cloned().collect())
    }

    fn scan_workspace_root(
        &self,
        root: &PathBuf,
        to_scan: &mut VecDeque<(String, usize)>,
        visited: &mut HashSet<String>,
    ) -> Result<()> {
        let manifest_path = root.join("Cargo.toml");
        if !manifest_path.exists() {
            return Ok(());
        }

        let metadata = MetadataCommand::new()
            .manifest_path(&manifest_path)
            .exec()
            .with_context(|| format!("Failed to get cargo metadata for {:?}", manifest_path))?;

        for package_id in &metadata.workspace_members {
            let package_id_str = package_id.to_string();
            if !visited.contains(&package_id_str) {
                to_scan.push_back((package_id_str, 0));
            }
        }

        Ok(())
    }

    fn resolve_crate_metadata(&self, package_id: &str, depth: usize) -> Result<Option<CrateMetadata>> {
        // Package IDs have format like: "path+file:///path/to/crate#version" or "registry+...#name@version"
        let (name, version, crate_path) = if let Some(hash_pos) = package_id.rfind('#') {
            let version_part = &package_id[hash_pos + 1..];
            let path_part = &package_id[..hash_pos];
            
            // Extract the actual file path from the package ID
            let actual_path = if path_part.starts_with("path+file://") {
                PathBuf::from(&path_part[12..]) // Remove "path+file://"
            } else {
                return Ok(None); // Skip non-path dependencies
            };
            
            // Extract name from version_part if it contains @
            let name = if let Some(at_pos) = version_part.rfind('@') {
                // Format: "name@version"
                version_part[..at_pos].to_string()
            } else {
                // Extract from path: get the last directory name
                if let Some(path_pos) = path_part.rfind('/') {
                    path_part[path_pos + 1..].to_string()
                } else {
                    return Ok(None);
                }
            };
            
            let version = if let Some(at_pos) = version_part.rfind('@') {
                version_part[at_pos + 1..].to_string()
            } else {
                version_part.to_string()
            };
            
            (name, version, actual_path)
        } else {
            return Ok(None);
        };

        if self.should_exclude_crate(&name) {
            return Ok(None);
        }

        // Use the path directly from package ID instead of searching
        let path = crate_path;
        let manifest_path = path.join("Cargo.toml");
        
        let (dependencies, is_workspace_member) = self.get_crate_dependencies(&manifest_path)?;
        let layer = self.get_crate_layer(&name);
        let is_external = depth > 0 && !is_workspace_member;

        Ok(Some(CrateMetadata {
            name,
            version,
            path,
            layer,
            depth,
            dependencies,
            is_workspace_member,
            is_external,
        }))
    }

    fn should_exclude_crate(&self, name: &str) -> bool {
        for pattern in &self.config.analysis.exclude_crates {
            if name.contains(pattern.trim_end_matches('*')) {
                return true;
            }
        }
        false
    }

    fn find_manifest_path(&self, crate_name: &str) -> Result<PathBuf> {
        for root in self.config.all_workspace_roots() {
            // Skip root workspace manifest - look for individual crate manifests only
            let potential_paths = [
                root.join(crate_name).join("Cargo.toml"),
                root.join("crates").join(crate_name).join("Cargo.toml"),
                root.join("libs").join(crate_name).join("Cargo.toml"),
            ];

            for path in &potential_paths {
                if path.exists() {
                    let metadata = MetadataCommand::new()
                        .manifest_path(path)
                        .exec()
                        .ok();
                    
                    if let Some(metadata) = metadata {
                        if metadata.packages.iter().any(|p| p.name == crate_name) {
                            return Ok(path.clone());
                        }
                    }
                }
            }
        }

        Err(anyhow::anyhow!("Could not find manifest for crate: {}", crate_name))
    }

    fn get_crate_dependencies(&self, manifest_path: &PathBuf) -> Result<(Vec<String>, bool)> {
        let metadata = MetadataCommand::new()
            .manifest_path(manifest_path)
            .exec()
            .with_context(|| format!("Failed to get metadata for {:?}", manifest_path))?;

        let mut dependencies = Vec::new();
        let is_workspace_member = !metadata.workspace_members.is_empty();

        for package in &metadata.packages {
            for dep in &package.dependencies {
                if self.should_include_dependency(dep) {
                    dependencies.push(dep.name.clone());
                }
            }
        }

        Ok((dependencies, is_workspace_member))
    }

    fn should_include_dependency(&self, dep: &cargo_metadata::Dependency) -> bool {
        use cargo_metadata::DependencyKind;

        match dep.kind {
            DependencyKind::Normal => true,
            DependencyKind::Development => self.config.analysis.include_dev_deps,
            DependencyKind::Build => self.config.analysis.include_build_deps,
            _ => false,
        }
    }

    fn get_crate_layer(&self, crate_name: &str) -> Option<String> {
        for layer in &self.config.architecture.layers {
            for crate_pattern in &layer.crates {
                if crate_pattern.ends_with('*') {
                    let prefix = crate_pattern.trim_end_matches('*');
                    if crate_name.starts_with(prefix) {
                        return Some(layer.name.clone());
                    }
                } else if crate_name == crate_pattern {
                    return Some(layer.name.clone());
                }
            }
        }
        None
    }

    pub fn get_discovered_crates(&self) -> &HashMap<String, CrateMetadata> {
        &self.discovered
    }

    pub fn get_workspace_members(&self) -> Vec<&CrateMetadata> {
        self.discovered.values()
            .filter(|crate_meta| crate_meta.is_workspace_member)
            .collect()
    }

    pub fn get_external_crates(&self) -> Vec<&CrateMetadata> {
        self.discovered.values()
            .filter(|crate_meta| crate_meta.is_external)
            .collect()
    }

    pub fn get_crates_by_layer(&self, layer: &str) -> Vec<&CrateMetadata> {
        self.discovered.values()
            .filter(|crate_meta| crate_meta.layer.as_ref() == Some(&layer.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.analysis.recursive_scan = true;
        config.analysis.include_dev_deps = false;
        config.analysis.exclude_crates = vec!["test-*".to_string()];
        config
    }

    #[tokio::test]
    async fn test_discover_workspace_crates() {
        let temp_dir = TempDir::new().unwrap();
        let workspace_root = temp_dir.path().to_path_buf();

        let cargo_toml = r#"
[package]
name = "test-workspace"
version = "0.1.0"
edition = "2021"

[workspace]
members = ["crate-a", "crate-b"]

[dependencies]
serde = "1.0"
"#;
        fs::write(workspace_root.join("Cargo.toml"), cargo_toml).unwrap();

        let mut config = create_test_config();
        config.workspace.root = workspace_root;

        let mut discovery = WorkspaceDiscovery::new(config);
        let crates = discovery.discover_crates().await.unwrap();

        assert!(!crates.is_empty());
        assert!(crates.iter().any(|c| c.name == "test-workspace"));
    }

    #[test]
    fn test_should_exclude_crate() {
        let config = create_test_config();
        let discovery = WorkspaceDiscovery::new(config);

        assert!(discovery.should_exclude_crate("test-helpers"));
        assert!(discovery.should_exclude_crate("test-utils"));
        assert!(!discovery.should_exclude_crate("my-crate"));
    }
}