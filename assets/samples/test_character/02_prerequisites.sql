-- test_character - Prerequisite Definitions
-- Generated: Thu Jul 16 08:44:47 CDT 2026

-- Branch 1 chain
INSERT INTO skill_node_prerequisites (
    skill_node_id,
    prereq_type,
    ref_node,
    sort_order
)
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.intermediate_skill'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.basic_foundation'),
    10;

INSERT INTO skill_node_prerequisites (
    skill_node_id,
    prereq_type,
    ref_node,
    sort_order
)
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.advanced_skill'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.intermediate_skill'),
    10;

-- Branch 2 chain
INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.alt_intermediate'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.alt_foundation'),
    10;

INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.alt_advanced'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.alt_intermediate'),
    10;

-- Branch 3 chain  
INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.spec_intermediate'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.spec_foundation'),
    10;

INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.spec_advanced'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = 'test_character.spec_intermediate'),
    10;
