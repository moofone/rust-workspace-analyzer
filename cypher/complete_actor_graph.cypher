// Complete actor and message visualization
// Shows all actors, all message types, and all relationships

// Option 1: Show everything - all actors, messages, and relationships
MATCH (a:Actor)
OPTIONAL MATCH (a)-[r:SENDS|HANDLES]-(m:MessageType)
RETURN a, r, m;

// Option 2: Show actors that can communicate 
// (any actor can potentially send to any actor that handles messages)
MATCH (receiver:Actor)-[:HANDLES]->(m:MessageType)
OPTIONAL MATCH (sender:Actor)-[:SENDS]->(m)
RETURN sender, receiver, m;

// Option 3: Show actual + potential communications
// This shows who currently sends messages and who could receive them
MATCH (m:MessageType)
OPTIONAL MATCH (sender:Actor)-[:SENDS]->(m)
OPTIONAL MATCH (receiver:Actor)-[:HANDLES]->(m)
RETURN sender, m, receiver;
