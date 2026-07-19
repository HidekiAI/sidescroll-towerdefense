-- test_character - Verification Queries
-- Generated: Thu Jul 16 08:44:47 CDT 2026

-- Verify entity and category
SELECT 'Entity and Category:' as section;
SELECT 
    e.key as entity,
    c.key as category
FROM entities e
CROSS JOIN skill_categories c
LEFT JOIN skill_category_grants scg ON scg.entity_id = e.id AND scg.skill_category_id = c.id
WHERE e.key = 'test_character' AND c.key = 'custom';

-- List skill nodes
SELECT 'Skill Nodes:' as section;
SELECT 
    sn.node_key,
    sn.max_level,
    sn.base_sp_cost,
    sn.sp_cost_per_level,
    sn.sort_order
FROM skill_nodes sn
JOIN entities e ON e.id = sn.entity_id
WHERE e.key = 'test_character'
ORDER BY sn.sort_order;

-- List prerequisites
SELECT 'Prerequisites:' as section;
SELECT 
    target.node_key as target_node,
    snp.prereq_type,
    parent.node_key as requires_node
FROM skill_node_prerequisites snp
JOIN skill_nodes target ON target.id = snp.skill_node_id
JOIN entities target_e ON target_e.id = target.entity_id
WHERE target_e.key = 'test_character'
ORDER BY target.node_key, snp.sort_order;

-- Cost summary
SELECT 'Cost Summary:' as section;
WITH node_costs AS (
    SELECT 
        sn.node_key,
        CASE 
            WHEN sn.max_level = 1 THEN sn.base_sp_cost
            ELSE sn.base_sp_cost + (sn.max_level - 1) * sn.sp_cost_per_level
        END as total_sp_cost
    FROM skill_nodes sn
    JOIN entities e ON e.id = sn.entity_id
    WHERE e.key = 'test_character'
)
SELECT 
    COUNT(*) as total_nodes,
    SUM(total_sp_cost) as total_sp_if_all_maxed,
    ROUND(AVG(total_sp_cost), 1) as average_cost_per_node
FROM node_costs;
