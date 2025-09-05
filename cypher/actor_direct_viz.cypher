// Simplified visualization showing just actors with virtual edges
// This creates virtual relationships directly between actors

MATCH (sender:Actor)-[s:SENDS]->(m:MessageType)<-[h:HANDLES]-(receiver:Actor)
WHERE sender.crate = receiver.crate
RETURN sender, receiver, 
       {type: "COMMUNICATES", message: m.name} as virtual_edge;
