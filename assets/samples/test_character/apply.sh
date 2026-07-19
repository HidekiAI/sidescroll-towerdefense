#!/bin/bash
# apply.sh - Apply SQL files to database
# Usage: ./apply.sh <database_file>

DB_FILE="$1"

if [ -z "$DB_FILE" ]; then
    echo "Usage: $0 <database_file>"
    exit 1
fi

echo "Applying test_character skill tree to: $DB_FILE"
echo ""

echo "Step 1: Entity and category setup"
sqlite3 "$DB_FILE" ".read 00_entity_setup.sql"

echo "Step 2: Skill node definitions"
sqlite3 "$DB_FILE" ".read 01_skill_nodes.sql"

echo "Step 3: Prerequisite chains"
sqlite3 "$DB_FILE" ".read 02_prerequisites.sql"

echo ""
echo "Step 4: Verification"
sqlite3 "$DB_FILE" ".read 03_verification.sql"

echo ""
echo "Done."
