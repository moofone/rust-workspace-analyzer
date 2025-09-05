// Check for SENDS relationships
MATCH ()-[s:SENDS]->()
RETURN count(s) as sends_count;

// Check for HANDLES relationships  
MATCH ()-[h:HANDLES]->()
RETURN count(h) as handles_count;

// Check for MessageType nodes
MATCH (m:MessageType)
RETURN count(m) as message_type_count;

// Check all relationship types
MATCH ()-[r]->()
RETURN DISTINCT type(r) as relationship_type, count(r) as count
ORDER BY count DESC;

// Check what relationships actors have
MATCH (a:Actor)-[r]->()
RETURN DISTINCT type(r) as rel_type, count(r) as count;

// Check if actors have any outgoing relationships
MATCH (a:Actor)
OPTIONAL MATCH (a)-[r]->()
RETURN a.name, count(r) as outgoing_rels
LIMIT 10;
