// Query to get all actors excluding test code
// Use this in Memgraph Lab to see only production actors

// Get all non-test actors with their properties
MATCH (a:Type:Actor)
WHERE a.is_test = false OR a.is_test IS NULL
RETURN a.name as actor_name, 
       a.crate as crate,
       a.is_distributed as is_distributed,
       a.actor_type as actor_type,
       a.file_path as file_path,
       a.is_test as is_test
ORDER BY a.crate, a.name;

// Count actors by test status
MATCH (a:Type:Actor)
RETURN 
  CASE WHEN a.is_test = true THEN 'Test Actors' ELSE 'Production Actors' END as category,
  count(a) as count
ORDER BY category;

// Get distributed actors excluding tests
MATCH (a:Type:Actor)
WHERE a.is_distributed = true 
  AND (a.is_test = false OR a.is_test IS NULL)
RETURN a.name as actor_name,
       a.crate as crate,
       a.distributed_messages,
       a.local_messages,
       a.file_path as file_path
ORDER BY a.crate, a.name;

// Get all actors with test status breakdown
MATCH (a:Type:Actor)
RETURN 
  a.crate as crate,
  sum(CASE WHEN a.is_test = true THEN 1 ELSE 0 END) as test_actors,
  sum(CASE WHEN a.is_test = false OR a.is_test IS NULL THEN 1 ELSE 0 END) as prod_actors,
  count(a) as total_actors
ORDER BY crate;