-- test_character - Entity and Category Setup
-- Generated: Thu Jul 16 08:44:47 CDT 2026

-- Create entity
INSERT INTO entities (key, name_id, entity_type) VALUES
    ('test_character', 'entity.test_character.name', 'support_staff');

-- Grant category (assumes category exists)
INSERT INTO skill_category_grants (entity_id, skill_category_id, min_rank_id)
SELECT 
    e.id,
    c.id,
    NULL
FROM entities e, skill_categories c
WHERE e.key = 'test_character' AND c.key = 'custom';
