// Check if Type:Actor nodes exist
MATCH (t:Type:Actor)
RETURN count(t) as type_actor_count;

// Check labels on Actor nodes
MATCH (a:Actor)
WHERE a.name = "CryptoFuturesDataDistributedActorLive"
RETURN labels(a) as labels;

// Check if there are Type nodes with HANDLES relationships
MATCH (t:Type)-[:HANDLES]->(m:MessageType)
RETURN t.name, m.name
LIMIT 10;
