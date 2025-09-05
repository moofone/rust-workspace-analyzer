// Simple Schema Query for Memgraph Lab
// Copy and paste each section separately for best results

// 1. Show all node types and one example
MATCH (n)
WITH labels(n) as nodeType, n
RETURN DISTINCT nodeType, head(collect(n)) as example
ORDER BY nodeType[0];

// 2. Show all relationship types with examples
MATCH (a)-[r]->(b)
RETURN DISTINCT 
  labels(a) as from_node,
  type(r) as relationship,
  labels(b) as to_node,
  keys(r) as relationship_properties
ORDER BY type(r);

// 3. Show Actor node properties (most complex node type)
MATCH (a:Type:Actor)
RETURN keys(a) as actor_properties, 
       a.is_distributed as is_distributed,
       a.distributed_messages,
       a.local_messages
LIMIT 5;

// 4. Count summary
MATCH (n)
WITH count(n) as totalNodes
MATCH ()-[r]->()
WITH totalNodes, count(r) as totalRelationships
MATCH (a:Type:Actor)
WITH totalNodes, totalRelationships, count(a) as actorCount
MATCH (a:Type:Actor) WHERE a.is_distributed = true
RETURN 
  totalNodes as total_nodes,
  totalRelationships as total_relationships,
  actorCount as total_actors,
  count(a) as distributed_actors;