#!/bin/bash
# generate.sh - Simple skill tree template generator (PoC)
# Creates SQL templates for new entities

set -e

ENTITY="$1"

if [ -z "$ENTITY" ]; then
    echo "Usage: $0 <entity_name>"
    echo ""
    echo "Creates: generated/<entity>/ with SQL templates"
    echo ""
    echo "Example: $0 time_mage"
    echo "Creates: generated/time_mage/"
    exit 1
fi

# Convert entity name to snake_case
ENTITY_SNAKE=$(echo "$ENTITY" | tr '-' '_' | tr ' ' '_' | tr '[:upper:]' '[:lower:]')
OUTPUT_DIR="generated/${ENTITY_SNAKE}"

echo "Generating skill tree template for: $ENTITY_SNAKE"
echo "Output: $OUTPUT_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# 1. Create README
cat > "$OUTPUT_DIR/README.txt" << EOF
$ENTITY Skill Tree Template
===========================

Generated: $(date)

Files to edit:
1. 00_entity_setup.sql    - Entity and category setup
2. 01_skill_nodes.sql     - Skill node definitions  
3. 02_prerequisites.sql   - Prerequisite chains

How to use:
1. Edit the SQL files (replace placeholder names and costs)
2. Validate: sqlite3 game.db "BEGIN; .read 00_entity_setup.sql; ROLLBACK;"
3. Apply: sqlite3 game.db ".read 00_entity_setup.sql"
            sqlite3 game.db ".read 01_skill_nodes.sql"
            sqlite3 game.db ".read 02_prerequisites.sql"
4. Verify: sqlite3 game.db ".read 03_verification.sql"

Template structure:
- 3 branches with A->B->C progression
- Foundation skills (A nodes): sort_order 10-19
- Intermediate skills (B nodes): sort_order 20-29  
- Advanced skills (C nodes): sort_order 30-39

Replace placeholder names:
  ${ENTITY_SNAKE}.basic_foundation -> Your actual skill name
  ${ENTITY_SNAKE}.intermediate_skill -> Your actual skill name
  ${ENTITY_SNAKE}.advanced_skill -> Your actual skill name

Update localization keys:
  skill.${ENTITY_SNAKE}.skill_name.name -> Display name
  skill.${ENTITY_SNAKE}.skill_name.desc -> Description
EOF

# 2. Entity setup SQL
cat > "$OUTPUT_DIR/00_entity_setup.sql" << EOF
-- $ENTITY_SNAKE - Entity and Category Setup
-- Generated: $(date)

-- Create entity
INSERT INTO entities (key, name_id, entity_type) VALUES
    ('${ENTITY_SNAKE}', 'entity.${ENTITY_SNAKE}.name', 'support_staff');

-- Grant category (assumes category exists)
INSERT INTO skill_category_grants (entity_id, skill_category_id, min_rank_id)
SELECT 
    e.id,
    c.id,
    NULL
FROM entities e, skill_categories c
WHERE e.key = '${ENTITY_SNAKE}' AND c.key = 'custom';
EOF

# 3. Skill nodes SQL  
cat > "$OUTPUT_DIR/01_skill_nodes.sql" << EOF
-- $ENTITY_SNAKE - Skill Node Definitions
-- Generated: $(date)

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
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.basic_foundation',
    'skill.${ENTITY_SNAKE}.basic_foundation.name',
    'skill.${ENTITY_SNAKE}.basic_foundation.desc',
    1,
    3,
    0,
    10;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.intermediate_skill',
    'skill.${ENTITY_SNAKE}.intermediate_skill.name',
    'skill.${ENTITY_SNAKE}.intermediate_skill.desc',
    3,
    4,
    2,
    20;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.advanced_skill',
    'skill.${ENTITY_SNAKE}.advanced_skill.name',
    'skill.${ENTITY_SNAKE}.advanced_skill.desc',
    1,
    8,
    0,
    30;

-- Branch 2: Alternative path
INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.alt_foundation',
    'skill.${ENTITY_SNAKE}.alt_foundation.name',
    'skill.${ENTITY_SNAKE}.alt_foundation.desc',
    1,
    3,
    0,
    11;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.alt_intermediate',
    'skill.${ENTITY_SNAKE}.alt_intermediate.name',
    'skill.${ENTITY_SNAKE}.alt_intermediate.desc',
    3,
    5,
    1,
    21;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.alt_advanced',
    'skill.${ENTITY_SNAKE}.alt_advanced.name',
    'skill.${ENTITY_SNAKE}.alt_advanced.desc',
    1,
    7,
    0,
    31;

-- Branch 3: Specialization
INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.spec_foundation',
    'skill.${ENTITY_SNAKE}.spec_foundation.name',
    'skill.${ENTITY_SNAKE}.spec_foundation.desc',
    5,
    4,
    2,
    12;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.spec_intermediate',
    'skill.${ENTITY_SNAKE}.spec_intermediate.name',
    'skill.${ENTITY_SNAKE}.spec_intermediate.desc',
    3,
    6,
    3,
    22;

INSERT INTO skill_nodes (...) 
SELECT 
    (SELECT id FROM entities WHERE key = '${ENTITY_SNAKE}'),
    (SELECT id FROM skill_categories WHERE key = 'custom'),
    '${ENTITY_SNAKE}.spec_advanced',
    'skill.${ENTITY_SNAKE}.spec_advanced.name',
    'skill.${ENTITY_SNAKE}.spec_advanced.desc',
    1,
    10,
    0,
    32;
EOF

# 4. Prerequisites SQL
cat > "$OUTPUT_DIR/02_prerequisites.sql" << EOF
-- $ENTITY_SNAKE - Prerequisite Definitions
-- Generated: $(date)

-- Branch 1 chain
INSERT INTO skill_node_prerequisites (
    skill_node_id,
    prereq_type,
    ref_node,
    sort_order
)
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.intermediate_skill'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.basic_foundation'),
    10;

INSERT INTO skill_node_prerequisites (
    skill_node_id,
    prereq_type,
    ref_node,
    sort_order
)
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.advanced_skill'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.intermediate_skill'),
    10;

-- Branch 2 chain
INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.alt_intermediate'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.alt_foundation'),
    10;

INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.alt_advanced'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.alt_intermediate'),
    10;

-- Branch 3 chain  
INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.spec_intermediate'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.spec_foundation'),
    10;

INSERT INTO skill_node_prerequisites (...) 
SELECT 
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.spec_advanced'),
    'branch_progression',
    (SELECT id FROM skill_nodes WHERE node_key = '${ENTITY_SNAKE}.spec_intermediate'),
    10;
EOF

# 5. Verification SQL
cat > "$OUTPUT_DIR/03_verification.sql" << EOF
-- $ENTITY_SNAKE - Verification Queries
-- Generated: $(date)

-- Verify entity and category
SELECT 'Entity and Category:' as section;
SELECT 
    e.key as entity,
    c.key as category
FROM entities e
CROSS JOIN skill_categories c
LEFT JOIN skill_category_grants scg ON scg.entity_id = e.id AND scg.skill_category_id = c.id
WHERE e.key = '${ENTITY_SNAKE}' AND c.key = 'custom';

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
WHERE e.key = '${ENTITY_SNAKE}'
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
WHERE target_e.key = '${ENTITY_SNAKE}'
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
    WHERE e.key = '${ENTITY_SNAKE}'
)
SELECT 
    COUNT(*) as total_nodes,
    SUM(total_sp_cost) as total_sp_if_all_maxed,
    ROUND(AVG(total_sp_cost), 1) as average_cost_per_node
FROM node_costs;
EOF

# 6. Simple apply script
cat > "$OUTPUT_DIR/apply.sh" << EOF
#!/bin/bash
# apply.sh - Apply SQL files to database
# Usage: ./apply.sh <database_file>

DB_FILE="\$1"

if [ -z "\$DB_FILE" ]; then
    echo "Usage: \$0 <database_file>"
    exit 1
fi

echo "Applying $ENTITY_SNAKE skill tree to: \$DB_FILE"
echo ""

echo "Step 1: Entity and category setup"
sqlite3 "\$DB_FILE" ".read 00_entity_setup.sql"

echo "Step 2: Skill node definitions"
sqlite3 "\$DB_FILE" ".read 01_skill_nodes.sql"

echo "Step 3: Prerequisite chains"
sqlite3 "\$DB_FILE" ".read 02_prerequisites.sql"

echo ""
echo "Step 4: Verification"
sqlite3 "\$DB_FILE" ".read 03_verification.sql"

echo ""
echo "Done."
EOF

chmod +x "$OUTPUT_DIR/apply.sh"

echo "Done."
echo "Files created in: $OUTPUT_DIR/"
echo ""
echo "Next:"
echo "1. cd $OUTPUT_DIR"
echo "2. Edit SQL files (replace placeholder names)"
echo "3. ./apply.sh path/to/game.db"
echo ""
echo "Note: Assumes 'custom' category exists. Edit 00_entity_setup.sql if using different category."