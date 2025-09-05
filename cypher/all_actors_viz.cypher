// Show ALL actors and their message relationships
// This includes both SENDS and HANDLES relationships

MATCH (a:Actor)
OPTIONAL MATCH (a)-[r:SENDS|HANDLES]->(m:MessageType)
RETURN a, r, m;
