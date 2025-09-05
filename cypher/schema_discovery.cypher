// Run this in Memgraph Lab to see complete schema

// Get all node labels with sample properties
CALL schema.node_type_properties() 
YIELD nodeType, nodeLabels, propertyName, propertyTypes, mandatory
WITH nodeLabels, collect({property: propertyName, types: propertyTypes, required: mandatory}) as properties
RETURN "NODE SCHEMA" as category, nodeLabels as labels, properties
ORDER BY nodeLabels[0];

// Get all relationship types with properties
CALL schema.rel_type_properties()
YIELD relType, propertyName, propertyTypes, mandatory  
WITH relType, collect({property: propertyName, types: propertyTypes, required: mandatory}) as properties
WHERE size(properties) > 0
RETURN "RELATIONSHIP SCHEMA" as category, relType, properties
ORDER BY relType;

// If the above procedures don't exist, use this alternative:
MATCH (n)
WITH DISTINCT labels(n) as nodeLabels, collect(DISTINCT keys(n)) as allKeys
UNWIND allKeys as keyList
UNWIND keyList as key
WITH nodeLabels, collect(DISTINCT key) as properties
RETURN nodeLabels, properties
ORDER BY nodeLabels[0];