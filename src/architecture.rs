use anyhow::Result;
use neo4rs::{Query, Row};
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::graph::MemgraphClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureViolation {
    pub kind: String,
    pub from: String,
    pub to: String,
    pub from_layer: String,
    pub to_layer: String,
    pub from_crate: String,
    pub to_crate: String,
    pub file: String,
    pub line: usize,
    pub severity: ViolationSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureReport {
    pub violations: Vec<ArchitectureViolation>,
    pub summary: ArchitectureSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureSummary {
    pub total_violations: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub violations_by_layer: std::collections::HashMap<String, usize>,
    pub most_problematic_crates: Vec<String>,
}

pub struct ArchitectureAnalyzer {
    graph: MemgraphClient,
    config: Config,
}

impl ArchitectureAnalyzer {
    pub fn new(graph: std::sync::Arc<MemgraphClient>, config: Config) -> Self {
        Self { graph: (*graph).clone(), config }
    }

    pub async fn analyze_architecture(&self) -> Result<ArchitectureReport> {
        let mut violations = Vec::new();

        violations.extend(self.check_layer_violations().await?);
        violations.extend(self.check_circular_dependencies().await?);
        violations.extend(self.check_dependency_direction().await?);
        violations.extend(self.check_public_api_violations().await?);

        let summary = self.generate_summary(&violations);

        Ok(ArchitectureReport {
            violations,
            summary,
        })
    }

    async fn check_layer_violations(&self) -> Result<Vec<ArchitectureViolation>> {
        let query = r#"
            MATCH (caller:Function)-[call:CALLS]->(callee:Function)
            WHERE caller.crate <> callee.crate
            WITH caller, callee, call,
                 caller.crate as from_crate,
                 callee.crate as to_crate
            MATCH (fc:Crate {name: from_crate})
            MATCH (tc:Crate {name: to_crate})
            WHERE fc.layer IS NOT NULL AND tc.layer IS NOT NULL
            RETURN caller, callee, call, fc.layer as from_layer, tc.layer as to_layer,
                   from_crate, to_crate
        "#;

        let result = self.graph.execute_query(Query::new(query.to_string())).await?;
        let mut violations = Vec::new();

        for row in result {
            if let Some(violation) = self.process_layer_violation_row(row).await? {
                violations.push(violation);
            }
        }

        eprintln!("🚨 Found {} layer violations", violations.len());
        Ok(violations)
    }

    async fn process_layer_violation_row(&self, row: Row) -> Result<Option<ArchitectureViolation>> {
        let from_layer: String = row.get("from_layer").unwrap_or_default();
        let to_layer: String = row.get("to_layer").unwrap_or_default();
        let from_crate: String = row.get("from_crate").unwrap_or_default();
        let to_crate: String = row.get("to_crate").unwrap_or_default();

        let from_layer_idx = self.config.get_layer_index(&from_layer);
        let to_layer_idx = self.config.get_layer_index(&to_layer);

        if let (Some(from_idx), Some(to_idx)) = (from_layer_idx, to_layer_idx) {
            if from_idx < to_idx {
                let caller_node: neo4rs::Node = row.get("caller").unwrap();
                let callee_node: neo4rs::Node = row.get("callee").unwrap();
                let call_rel: neo4rs::Relation = row.get("call").unwrap();
                
                let caller_name: String = caller_node.get("qualified_name").unwrap_or_default();
                let callee_name: String = callee_node.get("qualified_name").unwrap_or_default();
                let file: String = caller_node.get("file").unwrap_or_default();
                let line: i64 = call_rel.get("line").unwrap_or(0);

                return Ok(Some(ArchitectureViolation {
                    kind: "layer_violation".to_string(),
                    from: caller_name.clone(),
                    to: callee_name.clone(),
                    from_layer: from_layer.clone(),
                    to_layer: to_layer.clone(),
                    from_crate: from_crate.clone(),
                    to_crate: to_crate.clone(),
                    file: file.clone(),
                    line: line as usize,
                    severity: ViolationSeverity::Error,
                    message: format!(
                        "Layer violation: {} layer '{}' should not call {} layer '{}'. Function '{}' calls '{}'",
                        from_layer, from_crate, to_layer, to_crate, caller_name, callee_name
                    ),
                }));
            }
        }

        Ok(None)
    }

    async fn check_circular_dependencies(&self) -> Result<Vec<ArchitectureViolation>> {
        let crate_cycle_query = r#"
            MATCH (c1:Crate)-[:DEPENDS_ON*2..]->(c2:Crate)-[:DEPENDS_ON*1..]->(c1)
            RETURN DISTINCT c1.name as crate1, c2.name as crate2
        "#;

        let result = self.graph.execute_query(Query::new(crate_cycle_query.to_string())).await?;
        let mut violations = Vec::new();

        for row in result {
            let crate1: String = row.get("crate1").unwrap_or_default();
            let crate2: String = row.get("crate2").unwrap_or_default();

            violations.push(ArchitectureViolation {
                kind: "circular_dependency".to_string(),
                from: crate1.clone(),
                to: crate2.clone(),
                from_layer: "unknown".to_string(),
                to_layer: "unknown".to_string(),
                from_crate: crate1.clone(),
                to_crate: crate2.clone(),
                file: "".to_string(),
                line: 0,
                severity: ViolationSeverity::Error,
                message: format!("Circular dependency detected between crates '{}' and '{}'", crate1, crate2),
            });
        }

        eprintln!("🔄 Found {} circular dependencies", violations.len());
        Ok(violations)
    }

    async fn check_dependency_direction(&self) -> Result<Vec<ArchitectureViolation>> {
        let reverse_dep_query = r#"
            MATCH (lower:Crate)-[:DEPENDS_ON]->(higher:Crate)
            WHERE lower.layer IS NOT NULL AND higher.layer IS NOT NULL
            RETURN lower.name as lower_crate, lower.layer as lower_layer,
                   higher.name as higher_crate, higher.layer as higher_layer
        "#;

        let result = self.graph.execute_query(Query::new(reverse_dep_query.to_string())).await?;
        let mut violations = Vec::new();

        for row in result {
            let lower_layer: String = row.get("lower_layer").unwrap_or_default();
            let higher_layer: String = row.get("higher_layer").unwrap_or_default();
            let lower_crate: String = row.get("lower_crate").unwrap_or_default();
            let higher_crate: String = row.get("higher_crate").unwrap_or_default();

            if self.config.is_layer_violation(&lower_crate, &higher_crate) {
                violations.push(ArchitectureViolation {
                    kind: "reverse_dependency".to_string(),
                    from: lower_crate.clone(),
                    to: higher_crate.clone(),
                    from_layer: lower_layer.clone(),
                    to_layer: higher_layer.clone(),
                    from_crate: lower_crate.clone(),
                    to_crate: higher_crate.clone(),
                    file: "Cargo.toml".to_string(),
                    line: 0,
                    severity: ViolationSeverity::Error,
                    message: format!(
                        "Reverse dependency: Lower layer '{}' ({}) should not depend on higher layer '{}' ({})",
                        lower_layer, lower_crate, higher_layer, higher_crate
                    ),
                });
            }
        }

        eprintln!("⬆️ Found {} reverse dependencies", violations.len());
        Ok(violations)
    }

    async fn check_public_api_violations(&self) -> Result<Vec<ArchitectureViolation>> {
        let private_api_query = r#"
            MATCH (caller:Function)-[:CALLS]->(callee:Function)
            WHERE caller.crate <> callee.crate 
              AND callee.visibility <> "pub"
              AND NOT callee.visibility STARTS WITH "pub("
            RETURN caller.qualified_name as caller, caller.crate as caller_crate,
                   callee.qualified_name as callee, callee.crate as callee_crate,
                   callee.file as file, callee.line_start as line
        "#;

        let result = self.graph.execute_query(Query::new(private_api_query.to_string())).await?;
        let mut violations = Vec::new();

        for row in result {
            let caller: String = row.get("caller").unwrap_or_default();
            let caller_crate: String = row.get("caller_crate").unwrap_or_default();
            let callee: String = row.get("callee").unwrap_or_default();
            let callee_crate: String = row.get("callee_crate").unwrap_or_default();
            let file: String = row.get("file").unwrap_or_default();
            let line: i64 = row.get("line").unwrap_or(0);

            violations.push(ArchitectureViolation {
                kind: "private_api_access".to_string(),
                from: caller.clone(),
                to: callee.clone(),
                from_layer: "unknown".to_string(),
                to_layer: "unknown".to_string(),
                from_crate: caller_crate.clone(),
                to_crate: callee_crate.clone(),
                file: file.clone(),
                line: line as usize,
                severity: ViolationSeverity::Warning,
                message: format!(
                    "Private API access: '{}' in crate '{}' calls private function '{}' in crate '{}'",
                    caller, caller_crate, callee, callee_crate
                ),
            });
        }

        eprintln!("🔒 Found {} private API violations", violations.len());
        Ok(violations)
    }

    fn generate_summary(&self, violations: &[ArchitectureViolation]) -> ArchitectureSummary {
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut info_count = 0;
        let mut violations_by_layer = std::collections::HashMap::new();
        let mut crate_violation_counts = std::collections::HashMap::new();

        for violation in violations {
            match violation.severity {
                ViolationSeverity::Error => error_count += 1,
                ViolationSeverity::Warning => warning_count += 1,
                ViolationSeverity::Info => info_count += 1,
            }

            let layer_key = format!("{} -> {}", violation.from_layer, violation.to_layer);
            *violations_by_layer.entry(layer_key).or_insert(0) += 1;

            *crate_violation_counts.entry(violation.from_crate.clone()).or_insert(0) += 1;
        }

        let mut most_problematic_crates: Vec<(String, usize)> = crate_violation_counts.into_iter().collect();
        most_problematic_crates.sort_by(|a, b| b.1.cmp(&a.1));
        let most_problematic_crates: Vec<String> = most_problematic_crates
            .into_iter()
            .take(5)
            .map(|(crate_name, _)| crate_name)
            .collect();

        ArchitectureSummary {
            total_violations: violations.len(),
            error_count,
            warning_count,
            info_count,
            violations_by_layer,
            most_problematic_crates,
        }
    }

    pub async fn mark_violations_in_graph(&self, violations: &[ArchitectureViolation]) -> Result<()> {
        for violation in violations {
            if violation.kind == "layer_violation" {
                let mark_query = r#"
                    MATCH (caller:Function {qualified_name: $caller})
                    MATCH (callee:Function {qualified_name: $callee})
                    MATCH (caller)-[r:CALLS]->(callee)
                    SET r.violates_architecture = true,
                        r.violation_kind = $kind,
                        r.violation_severity = $severity
                "#;

                let query = Query::new(mark_query.to_string())
                    .param("caller", violation.from.clone())
                    .param("callee", violation.to.clone())
                    .param("kind", violation.kind.clone())
                    .param("severity", format!("{:?}", violation.severity));

                let _ = self.graph.execute_query(query).await;
            }
        }

        eprintln!("✅ Marked {} violations in graph", violations.len());
        Ok(())
    }

    pub async fn get_violations_for_function(&self, function_name: &str) -> Result<Vec<ArchitectureViolation>> {
        let query = r#"
            MATCH (f:Function {qualified_name: $function_name})
            MATCH (f)-[r:CALLS {violates_architecture: true}]->(target:Function)
            RETURN f.qualified_name as caller, target.qualified_name as callee,
                   f.crate as caller_crate, target.crate as callee_crate,
                   f.file as file, r.line as line,
                   r.violation_kind as kind, r.violation_severity as severity
        "#;

        let query_obj = Query::new(query.to_string())
            .param("function_name", function_name);

        let result = self.graph.execute_query(query_obj).await?;
        let mut violations = Vec::new();

        for row in result {
            let caller: String = row.get("caller").unwrap_or_default();
            let callee: String = row.get("callee").unwrap_or_default();
            let caller_crate: String = row.get("caller_crate").unwrap_or_default();
            let callee_crate: String = row.get("callee_crate").unwrap_or_default();
            let file: String = row.get("file").unwrap_or_default();
            let line: i64 = row.get("line").unwrap_or(0);
            let kind: String = row.get("kind").unwrap_or_default();
            let severity_str: String = row.get("severity").unwrap_or_default();

            let severity = match severity_str.as_str() {
                "Error" => ViolationSeverity::Error,
                "Warning" => ViolationSeverity::Warning,
                _ => ViolationSeverity::Info,
            };

            violations.push(ArchitectureViolation {
                kind,
                from: caller.clone(),
                to: callee.clone(),
                from_layer: "unknown".to_string(),
                to_layer: "unknown".to_string(),
                from_crate: caller_crate.clone(),
                to_crate: callee_crate.clone(),
                file,
                line: line as usize,
                severity,
                message: format!("Function '{}' violates architecture by calling '{}'", caller, callee),
            });
        }

        Ok(violations)
    }

    pub async fn get_layer_health(&self) -> Result<std::collections::HashMap<String, LayerHealth>> {
        let mut layer_health = std::collections::HashMap::new();

        for layer in &self.config.architecture.layers {
            let health = self.calculate_layer_health(&layer.name).await?;
            layer_health.insert(layer.name.clone(), health);
        }

        Ok(layer_health)
    }

    async fn calculate_layer_health(&self, layer_name: &str) -> Result<LayerHealth> {
        let violations_query = r#"
            MATCH (c:Crate {layer: $layer})
            MATCH (f:Function {crate: c.name})
            MATCH (f)-[r:CALLS {violates_architecture: true}]->()
            RETURN count(r) as violations
        "#;

        let query = Query::new(violations_query.to_string())
            .param("layer", layer_name);

        let result = self.graph.execute_query(query).await?;
        let violations = if let Some(row) = result.first() {
            row.get::<i64>("violations").unwrap_or(0) as usize
        } else {
            0
        };

        let health_score = if violations == 0 {
            100.0
        } else {
            100.0 / (1.0 + violations as f64 / 10.0)
        };

        Ok(LayerHealth {
            layer: layer_name.to_string(),
            violation_count: violations,
            health_score,
            status: if health_score >= 90.0 {
                "healthy".to_string()
            } else if health_score >= 70.0 {
                "warning".to_string()
            } else {
                "critical".to_string()
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerHealth {
    pub layer: String,
    pub violation_count: usize,
    pub health_score: f64,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Layer};

    fn create_test_config() -> Config {
        let mut config = Config::default();
        config.architecture.layers = vec![
            Layer { name: "core".to_string(), crates: vec!["core-lib".to_string()] },
            Layer { name: "domain".to_string(), crates: vec!["domain-lib".to_string()] },
            Layer { name: "app".to_string(), crates: vec!["app-lib".to_string()] },
        ];
        config
    }

    #[test]
    fn test_layer_violation_detection() {
        let config = create_test_config();
        
        assert!(config.is_layer_violation("core-lib", "domain-lib"));
        assert!(config.is_layer_violation("core-lib", "app-lib"));
        assert!(config.is_layer_violation("domain-lib", "app-lib"));
        
        assert!(!config.is_layer_violation("app-lib", "domain-lib"));
        assert!(!config.is_layer_violation("domain-lib", "core-lib"));
        assert!(!config.is_layer_violation("app-lib", "core-lib"));
    }

    #[test]
    fn test_violation_severity() {
        let violation = ArchitectureViolation {
            kind: "layer_violation".to_string(),
            from: "test_fn".to_string(),
            to: "other_fn".to_string(),
            from_layer: "core".to_string(),
            to_layer: "app".to_string(),
            from_crate: "core-lib".to_string(),
            to_crate: "app-lib".to_string(),
            file: "test.rs".to_string(),
            line: 10,
            severity: ViolationSeverity::Error,
            message: "Test violation".to_string(),
        };

        assert!(matches!(violation.severity, ViolationSeverity::Error));
    }
}