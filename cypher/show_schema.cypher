// Comprehensive Schema Discovery for Rust Workspace Analyzer Graph

// ============================================
// 1. NODE TYPES AND THEIR PROPERTIES
// ============================================

// Show all distinct node labels and count
MATCH (n)
WITH DISTINCT labels(n) as nodeLabels, count(n) as nodeCount
RETURN "NODE TYPES" as Category, nodeLabels as Labels, nodeCount as Count
ORDER BY nodeCount DESC;

// Show properties for each node type
MATCH (n)
WITH DISTINCT labels(n) as nodeLabels, n
WITH nodeLabels, keys(n) as properties
RETURN "NODE PROPERTIES" as Category, nodeLabels as NodeType, collect(DISTINCT properties) as Properties
ORDER BY nodeLabels;

// ============================================
// 2. RELATIONSHIP TYPES AND THEIR PROPERTIES
// ============================================

// Show all relationship types with counts
MATCH ()-[r]->()
WITH type(r) as relType, count(r) as relCount
RETURN "RELATIONSHIP TYPES" as Category, relType as Type, relCount as Count
ORDER BY relCount DESC;

// Show properties for each relationship type
MATCH ()-[r]->()
WITH type(r) as relType, keys(r) as properties
RETURN "RELATIONSHIP PROPERTIES" as Category, relType as RelationshipType, collect(DISTINCT properties) as Properties
ORDER BY relType;

// ============================================
// 3. DETAILED NODE SCHEMA
// ============================================

// Function nodes
MATCH (f:Function)
WITH f LIMIT 1
RETURN "SCHEMA: Function" as NodeType, keys(f) as Properties;

// Type nodes (including Actors)
MATCH (t:Type)
WITH t LIMIT 1
RETURN "SCHEMA: Type" as NodeType, keys(t) as Properties;

// Actor nodes (Type:Actor dual label)
MATCH (a:Type:Actor)
WITH a LIMIT 1
RETURN "SCHEMA: Type:Actor" as NodeType, keys(a) as Properties;

// Module nodes
MATCH (m:Module)
WITH m LIMIT 1
RETURN "SCHEMA: Module" as NodeType, keys(m) as Properties;

// MessageType nodes
MATCH (mt:MessageType)
WITH mt LIMIT 1
RETURN "SCHEMA: MessageType" as NodeType, keys(mt) as Properties;

// ============================================
// 4. RELATIONSHIP PATTERNS
// ============================================

// Show actual relationship patterns (who connects to whom)
MATCH (a)-[r]->(b)
WITH labels(a) as fromLabels, type(r) as relType, labels(b) as toLabels, count(*) as count
RETURN "RELATIONSHIP PATTERNS" as Category, 
       fromLabels as From, 
       relType as Relationship, 
       toLabels as To, 
       count as Count
ORDER BY count DESC
LIMIT 20;

// ============================================
// 5. ACTOR-SPECIFIC SCHEMA (Local vs Distributed)
// ============================================

// Show distributed actors
MATCH (a:Type:Actor)
WHERE a.is_distributed = true
WITH count(a) as distributedCount
MATCH (a:Type:Actor)
WHERE a.is_distributed = false OR a.is_distributed IS NULL
WITH distributedCount, count(a) as localCount
RETURN "ACTOR DISTRIBUTION" as Category, distributedCount as DistributedActors, localCount as LocalOnlyActors;

// Show distributed message flows
MATCH ()-[r:SENDS_DISTRIBUTED]->()
WITH count(r) as distFlows
MATCH ()-[r:SENDS]->()
WITH distFlows, count(r) as localFlows
RETURN "MESSAGE FLOWS" as Category, distFlows as DistributedFlows, localFlows as LocalFlows;

// ============================================
// 6. SAMPLE DATA FOR EACH NODE TYPE
// ============================================

// Sample Function node
MATCH (f:Function)
RETURN "SAMPLE: Function" as Type, f
LIMIT 1;

// Sample Type:Actor node with distributed flag
MATCH (a:Type:Actor)
WHERE a.is_distributed = true
RETURN "SAMPLE: Distributed Actor" as Type, a
LIMIT 1;

// Sample MessageType node
MATCH (m:MessageType)
RETURN "SAMPLE: MessageType" as Type, m
LIMIT 1;

// Sample SPAWNS relationship
MATCH (a)-[r:SPAWNS]->(b)
RETURN "SAMPLE: SPAWNS" as Type, r
LIMIT 1;

// Sample SENDS_DISTRIBUTED relationship
MATCH (a)-[r:SENDS_DISTRIBUTED]->(b)
RETURN "SAMPLE: SENDS_DISTRIBUTED" as Type, r
LIMIT 1;