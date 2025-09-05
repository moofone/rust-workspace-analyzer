-- Check if synthetic CALLS relationships exist
MATCH ()-[r:CALLS {is_synthetic: true}]->()
RETURN count(r) as synthetic_call_count;

-- Check specific indicator functions
MATCH (f:Function)
WHERE f.name = 'new' AND f.crate = 'trading_ta'
RETURN f.qualified_name, f.module
LIMIT 10;

-- Check if ANY calls target indicator new() functions
MATCH (f:Function)<-[r:CALLS]-()
WHERE f.name = 'new' AND f.crate = 'trading_ta'
RETURN f.qualified_name, count(r) as call_count
ORDER BY f.qualified_name;

-- Check what synthetic calls were created
MATCH (caller:Function)-[r:CALLS {is_synthetic: true}]->(target)
RETURN caller.name as caller, target.qualified_name as target
LIMIT 20;