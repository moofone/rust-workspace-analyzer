// Return actual graph elements for Memgraph visualization
// This will show actors as nodes and their message flows as edges

MATCH (sender:Actor)-[s:SENDS]->(m:MessageType)<-[h:HANDLES]-(receiver:Actor)
WHERE sender.crate = receiver.crate
RETURN sender, s, m, h, receiver;
