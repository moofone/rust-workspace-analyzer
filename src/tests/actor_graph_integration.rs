use crate::graph::memgraph_client::MemgraphClient;
use crate::parser::types::{RustActor, ActorImplementationType, ActorSpawn, SpawnMethod, SpawnPattern};
use neo4rs::Query;
use anyhow::Result;

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_client() -> Result<MemgraphClient> {
        // Create a test client (would need to be configured for test environment)
        let client = MemgraphClient::new("bolt://localhost:7687", "", "").await?;
        // Clear any existing test data
        client.clear_workspace().await?;
        Ok(client)
    }

    #[tokio::test]
    async fn test_no_duplicate_actor_nodes() -> Result<()> {
        let client = setup_test_client().await?;

        // First, create a Type node
        let query = Query::new("CREATE (t:Type {
            id: 'test:MyActor',
            name: 'MyActor',
            crate: 'test_crate',
            kind: 'struct'
        })".to_string());
        client.graph.run(query).await?;

        // Now create an actor for the same struct
        let actor = RustActor {
            id: "test:MyActor".to_string(),
            name: "MyActor".to_string(),
            qualified_name: "test_crate::MyActor".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: "test_crate".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 10,
            line_end: 20,
            visibility: "pub".to_string(),
            doc_comment: None,
            is_distributed: true,
            actor_type: ActorImplementationType::KameoActor,
        };

        client.create_actor_nodes(&[actor]).await?;

        // Verify there's only one node with both labels
        let verify_query = Query::new("MATCH (n:Type:Actor {name: 'MyActor', crate: 'test_crate'}) RETURN count(n) as count".to_string());
        let mut result = client.graph.execute(verify_query).await?;
        
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            assert_eq!(count, 1, "Should have exactly one Type:Actor node");
        }

        // Verify no standalone Actor nodes exist
        let actor_query = Query::new("MATCH (a:Actor) WHERE NOT a:Type RETURN count(a) as count".to_string());
        let mut result = client.graph.execute(actor_query).await?;
        
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            assert_eq!(count, 0, "Should have no standalone Actor nodes");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_actor_methods_connected() -> Result<()> {
        let client = setup_test_client().await?;

        // Create an actor with Type:Actor labels
        let actor = RustActor {
            id: "test:MessageHandler".to_string(),
            name: "MessageHandler".to_string(),
            qualified_name: "test_crate::MessageHandler".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: "test_crate".to_string(),
            file_path: "src/lib.rs".to_string(),
            line_start: 30,
            line_end: 50,
            visibility: "pub".to_string(),
            doc_comment: None,
            is_distributed: false,
            actor_type: ActorImplementationType::KameoActor,
        };

        client.create_actor_nodes(&[actor]).await?;

        // Create a method for the actor
        let method_query = Query::new("
            MATCH (a:Type:Actor {name: 'MessageHandler', crate: 'test_crate'})
            CREATE (m:Function {
                id: 'test:MessageHandler::handle',
                name: 'handle',
                qualified_name: 'test_crate::MessageHandler::handle'
            })
            CREATE (a)-[:HAS_METHOD]->(m)
        ".to_string());
        client.graph.run(method_query).await?;

        // Verify the method is connected
        let verify_query = Query::new("
            MATCH (a:Type:Actor {name: 'MessageHandler'})-[:HAS_METHOD]->(m:Function)
            RETURN count(m) as method_count
        ".to_string());
        let mut result = client.graph.execute(verify_query).await?;
        
        if let Some(row) = result.next().await? {
            let method_count: i64 = row.get("method_count")?;
            assert_eq!(method_count, 1, "Actor should have one connected method");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_spawn_relationships_use_type_nodes() -> Result<()> {
        let client = setup_test_client().await?;

        // Create parent and child actors
        let parent_actor = RustActor {
            id: "test:ParentActor".to_string(),
            name: "ParentActor".to_string(),
            qualified_name: "test_crate::ParentActor".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: "test_crate".to_string(),
            file_path: "src/parent.rs".to_string(),
            line_start: 1,
            line_end: 20,
            visibility: "pub".to_string(),
            doc_comment: None,
            is_distributed: true,
            actor_type: ActorImplementationType::KameoActor,
        };

        let child_actor = RustActor {
            id: "test:ChildActor".to_string(),
            name: "ChildActor".to_string(),
            qualified_name: "test_crate::ChildActor".to_string(),
            crate_name: "test_crate".to_string(),
            module_path: "test_crate".to_string(),
            file_path: "src/child.rs".to_string(),
            line_start: 1,
            line_end: 15,
            visibility: "pub".to_string(),
            doc_comment: None,
            is_distributed: false,
            actor_type: ActorImplementationType::KameoActor,
        };

        client.create_actor_nodes(&[parent_actor, child_actor]).await?;

        // Create spawn relationship
        let spawn = ActorSpawn {
            parent_actor_name: "ParentActor".to_string(),
            child_actor_name: "ChildActor".to_string(),
            from_crate: "test_crate".to_string(),
            to_crate: "test_crate".to_string(),
            spawn_method: SpawnMethod::Spawn,
            context: "spawns child actor".to_string(),
            line: 10,
            file_path: "src/parent.rs".to_string(),
            spawn_pattern: SpawnPattern::DirectSpawn,
        };

        client.create_actor_spawn_relationships(&[spawn]).await?;

        // Verify spawn relationship connects Type:Actor nodes
        let verify_query = Query::new("
            MATCH (parent:Type:Actor {name: 'ParentActor'})-[r:SPAWNS]->(child:Type:Actor {name: 'ChildActor'})
            RETURN count(r) as spawn_count
        ".to_string());
        let mut result = client.graph.execute(verify_query).await?;
        
        if let Some(row) = result.next().await? {
            let spawn_count: i64 = row.get("spawn_count")?;
            assert_eq!(spawn_count, 1, "Should have one spawn relationship between Type:Actor nodes");
        }

        // Verify no relationships to standalone Actor nodes
        let old_style_query = Query::new("
            MATCH (a:Actor)-[r:SPAWNS]-(b:Actor)
            WHERE NOT a:Type OR NOT b:Type
            RETURN count(r) as old_count
        ".to_string());
        let mut result = client.graph.execute(old_style_query).await?;
        
        if let Some(row) = result.next().await? {
            let old_count: i64 = row.get("old_count")?;
            assert_eq!(old_count, 0, "Should have no spawn relationships involving standalone Actor nodes");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_migration_preserves_data() -> Result<()> {
        let client = setup_test_client().await?;

        // Create old-style Actor node
        let create_old_actor = Query::new("CREATE (a:Actor {
            id: 'old:OldActor',
            name: 'OldActor',
            crate: 'old_crate',
            is_distributed: true,
            actor_type: 'KameoActor'
        })".to_string());
        client.graph.run(create_old_actor).await?;

        // Create a Type node for the same entity
        let create_type = Query::new("CREATE (t:Type {
            id: 'old:OldActor',
            name: 'OldActor',
            crate: 'old_crate',
            kind: 'struct'
        })".to_string());
        client.graph.run(create_type).await?;

        // Run migration
        client.migrate_actors_to_type_nodes().await?;

        // Verify Type node has Actor label and properties
        let verify_query = Query::new("
            MATCH (t:Type:Actor {name: 'OldActor', crate: 'old_crate'})
            RETURN t.is_distributed as distributed, t.actor_type as actor_type
        ".to_string());
        let mut result = client.graph.execute(verify_query).await?;
        
        if let Some(row) = result.next().await? {
            let distributed: bool = row.get("distributed")?;
            let actor_type: String = row.get("actor_type")?;
            assert_eq!(distributed, true, "is_distributed should be preserved");
            assert_eq!(actor_type, "KameoActor", "actor_type should be preserved");
        }

        // Verify old Actor node is deleted
        let old_actor_query = Query::new("MATCH (a:Actor {name: 'OldActor'}) RETURN count(a) as count".to_string());
        let mut result = client.graph.execute(old_actor_query).await?;
        
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            assert_eq!(count, 0, "Old Actor node should be deleted");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_actor_queries_work_with_multi_labels() -> Result<()> {
        let client = setup_test_client().await?;

        // Create actors
        let actors = vec![
            RustActor {
                id: "test:Actor1".to_string(),
                name: "Actor1".to_string(),
                qualified_name: "test::Actor1".to_string(),
                crate_name: "test".to_string(),
                module_path: "test".to_string(),
                file_path: "src/lib.rs".to_string(),
                line_start: 1,
                line_end: 10,
                visibility: "pub".to_string(),
                doc_comment: None,
                is_distributed: true,
                actor_type: ActorImplementationType::KameoActor,
            },
            RustActor {
                id: "test:Actor2".to_string(),
                name: "Actor2".to_string(),
                qualified_name: "test::Actor2".to_string(),
                crate_name: "test".to_string(),
                module_path: "test".to_string(),
                file_path: "src/lib.rs".to_string(),
                line_start: 20,
                line_end: 30,
                visibility: "pub".to_string(),
                doc_comment: None,
                is_distributed: false,
                actor_type: ActorImplementationType::BasicActor,
            },
        ];

        client.create_actor_nodes(&actors).await?;

        // Query using Type:Actor pattern
        let query = Query::new("MATCH (a:Type:Actor) RETURN count(a) as count".to_string());
        let mut result = client.graph.execute(query).await?;
        
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            assert_eq!(count, 2, "Should find both actors using Type:Actor pattern");
        }

        // Query distributed actors
        let dist_query = Query::new("MATCH (a:Type:Actor {is_distributed: true}) RETURN count(a) as count".to_string());
        let mut result = client.graph.execute(dist_query).await?;
        
        if let Some(row) = result.next().await? {
            let count: i64 = row.get("count")?;
            assert_eq!(count, 1, "Should find one distributed actor");
        }

        Ok(())
    }
}