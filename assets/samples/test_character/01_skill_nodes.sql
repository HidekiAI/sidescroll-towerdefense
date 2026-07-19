-- test_character - Skill Node Definitions
-- Generated: Thu Jul 16 08:44:47 CDT 2026

-- Branch 1: Foundation -> Intermediate -> Advanced
INSERT INTO skill_nodes (
    entity_id,
    category_id,
    node_key,
    display_name_id,
    description_id,
    max_level,
    base_sp_cost,
    sp_cost_per_level,
    sort_order
) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.basic_foundation',
    'skill.test_character.basic_foundation.name',
    'skill.test_character.basic_foundation.desc',
    1,
    3,
    0,
    10;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.intermediate_skill',
    'skill.test_character.intermediate_skill.name',
    'skill.test_character.intermediate_skill.desc',
    3,
    4,
    2,
    20;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.advanced_skill',
    'skill.test_character.advanced_skill.name',
    'skill.test_character.advanced_skill.desc',
    1,
    8,
    0,
    30;

-- Branch 2: Alternative path
INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.alt_foundation',
    'skill.test_character.alt_foundation.name',
    'skill.test_character.alt_foundation.desc',
    1,
    3,
    0,
    11;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.alt_intermediate',
    'skill.test_character.alt_intermediate.name',
    'skill.test_character.alt_intermediate.desc',
    3,
    5,
    1,
    21;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.alt_advanced',
    'skill.test_character.alt_advanced.name',
    'skill.test_character.alt_advanced.desc',
    1,
    7,
    0,
    31;

-- Branch 3: Specialization
INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.spec_foundation',
    'skill.test_character.spec_foundation.name',
    'skill.test_character.spec_foundation.desc',
    5,
    4,
    2,
    12;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.spec_intermediate',
    'skill.test_character.spec_intermediate.name',
    'skill.test_character.spec_intermediate.desc',
    3,
    6,
    3,
    22;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = 'test_character'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    'test_character.spec_advanced',
    'skill.test_character.spec_advanced.name',
    'skill.test_character.spec_advanced.desc',
    1,
    10,
    0,
    32;
