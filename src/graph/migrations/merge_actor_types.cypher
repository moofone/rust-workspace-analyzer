// Migration script to merge Actor nodes into Type nodes with multi-label approach
// This script migrates existing graphs from separate Actor nodes to Type:Actor multi-label nodes

// Step 1: Find all existing Actor nodes and corresponding Type nodes
// Add Actor label to existing Type nodes
MATCH (a:Actor)
OPTIONAL MATCH (t:Type {name: a.name, crate: a.crate})
WHERE t IS NOT NULL
SET t:Actor,
    t.is_distributed = COALESCE(a.is_distributed, false),
    t.actor_type = COALESCE(a.actor_type, 'Unknown')
WITH a, t
WHERE t IS NOT NULL
RETURN count(t) as types_updated;

// Step 2: Create Type:Actor nodes for Actors without corresponding Type nodes
MATCH (a:Actor)
WHERE NOT EXISTS {
    MATCH (t:Type {name: a.name, crate: a.crate})
}
CREATE (t:Type:Actor {
    id: a.id,
    name: a.name,
    qualified_name: a.qualified_name,
    crate: a.crate,
    module_path: a.module_path,
    file_path: a.file_path,
    line_start: a.line_start,
    line_end: a.line_end,
    visibility: a.visibility,
    is_distributed: COALESCE(a.is_distributed, false),
    actor_type: COALESCE(a.actor_type, 'Unknown'),
    kind: 'struct'
})
WITH a, t
RETURN count(t) as new_type_actors_created;

// Step 3: Transfer SPAWNS relationships from Actor nodes to Type:Actor nodes
MATCH (parent_actor:Actor)-[r:SPAWNS]->(child_actor:Actor)
MATCH (parent_type:Type:Actor {name: parent_actor.name, crate: parent_actor.crate})
MATCH (child_type:Type:Actor {name: child_actor.name, crate: child_actor.crate})
CREATE (parent_type)-[:SPAWNS {
    method: r.method,
    context: r.context,
    line: r.line,
    file_path: r.file_path,
    spawn_pattern: r.spawn_pattern
}]->(child_type)
DELETE r
WITH count(r) as spawns_transferred
RETURN spawns_transferred;

// Step 4: Transfer SENDS relationships
MATCH (sender_actor:Actor)-[r:SENDS]->(receiver_actor:Actor)
MATCH (sender_type:Type:Actor {name: sender_actor.name, crate: sender_actor.crate})
MATCH (receiver_type:Type:Actor {name: receiver_actor.name, crate: receiver_actor.crate})
CREATE (sender_type)-[:SENDS {
    message_type: r.message_type,
    method: r.method,
    line: r.line,
    file_path: r.file_path
}]->(receiver_type)
DELETE r
WITH count(r) as sends_transferred
RETURN sends_transferred;

// Step 5: Transfer HANDLES relationships
MATCH (actor:Actor)-[r:HANDLES]->(msg:MessageType)
MATCH (type:Type:Actor {name: actor.name, crate: actor.crate})
CREATE (type)-[:HANDLES {
    reply_type: r.reply_type,
    is_async: r.is_async,
    line: r.line,
    file_path: r.file_path
}]->(msg)
DELETE r
WITH count(r) as handles_transferred
RETURN handles_transferred;

// Step 6: Delete old Actor nodes
// IMPORTANT: Only run this after verifying all relationships have been transferred
MATCH (a:Actor)
DETACH DELETE a
RETURN count(a) as actors_deleted;

// Step 7: Verify migration results
MATCH (t:Type:Actor)
RETURN count(t) as total_type_actors;

MATCH ()-[r:SPAWNS]->()
RETURN count(r) as total_spawn_relationships;

MATCH ()-[r:SENDS]->()
RETURN count(r) as total_send_relationships;

MATCH ()-[r:HANDLES]->()
RETURN count(r) as total_handle_relationships;

// Optional: Create rollback script (save original data first)
// This would require exporting the Actor nodes and relationships before migration