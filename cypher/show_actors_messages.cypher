// Show all actors and the messages they handle (local messages)
MATCH (a:Actor)-[h:HANDLES]->(m:MessageType)
RETURN a.crate as crate, a.name as actor, collect(m.name) as handles_messages
ORDER BY a.crate, a.name;
