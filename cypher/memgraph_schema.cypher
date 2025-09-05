// Complete Schema Discovery - Run in Memgraph Lab
// Returns node types, relationship types, and their properties

WITH 1 as dummy

// Get node schema
MATCH (n)
WITH DISTINCT labels(n) as nodeLabels, n
WITH nodeLabels, collect(DISTINCT keys(n)) as propertyLists
UNWIND propertyLists as props
UNWIND props as prop
WITH nodeLabels, collect(DISTINCT prop) as properties
WITH collect({labels: nodeLabels, properties: properties}) as nodeSchema

// Get relationship schema  
MATCH ()-[r]->()
WITH nodeSchema, DISTINCT type(r) as relType, r
WITH nodeSchema, relType, collect(DISTINCT keys(r)) as propertyLists
UNWIND propertyLists as props
UNWIND props as prop  
WITH nodeSchema, relType, collect(DISTINCT prop) as properties
WITH nodeSchema, collect({type: relType, properties: properties}) as relSchema

// Get relationship patterns
MATCH (a)-[r]->(b)
WITH nodeSchema, relSchema, labels(a) as fromLabels, type(r) as relType, labels(b) as toLabels
WITH nodeSchema, relSchema, collect(DISTINCT {from: fromLabels, relationship: relType, to: toLabels}) as patterns

RETURN nodeSchema, relSchema, patterns;