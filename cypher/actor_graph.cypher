// Create a graph visualization query for actors and their communications
// This returns nodes and edges in a format suitable for visualization

// Get all actors as nodes
MATCH (a:Actor)
WHERE a.crate IN ['trading-strategy', 'data-services', 'trading-exchanges']
WITH collect({id: a.name, label: a.name, crate: a.crate, type: 'Actor'}) as nodes

// Get all message flows as edges  
MATCH (sender:Actor)-[s:SENDS]->(m:MessageType)<-[h:HANDLES]-(receiver:Actor)
WHERE sender.crate = receiver.crate
WITH nodes, collect({
    from: sender.name, 
    to: receiver.name, 
    label: m.name,
    type: 'SENDS_MESSAGE'
}) as edges

RETURN nodes, edges;
