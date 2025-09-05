// Check detailed SENDS relationships
MATCH (a1:Actor)-[s:SENDS]->(m:MessageType)
RETURN a1.name as sender, m.name as message, a1.crate as crate
ORDER BY sender, message;

// Check if any actors receive messages
MATCH (m:MessageType)<-[h:HANDLES]-(a:Actor)
RETURN count(h) as handles_count;

// Check MessageType nodes details
MATCH (m:MessageType)
RETURN m.name, m.crate
LIMIT 10;
