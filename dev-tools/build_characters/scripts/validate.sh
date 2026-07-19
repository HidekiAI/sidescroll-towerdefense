#!/bin/bash
# validate.sh - Simple SQL validation
# Usage: ./validate.sh <sql_file> <db_file>

set -e

SQL_FILE="$1"
DB_FILE="$2"

if [ -z "$SQL_FILE" ] || [ -z "$DB_FILE" ]; then
    echo "Usage: $0 <sql_file> <db_file>"
    echo "Example: $0 new_skills.sql game.db"
    exit 1
fi

echo "Validating: $SQL_FILE"
echo "Database: $DB_FILE"
echo "========================================="

# 1. Check SQL syntax
echo "1. Checking SQL syntax..."
if ! sqlite3 "$DB_FILE" "BEGIN; .read '$SQL_FILE'; ROLLBACK;" 2>/tmp/sql_error; then
    echo "ERROR: SQL syntax error"
    echo "Details:"
    cat /tmp/sql_error
    rm /tmp/sql_error
    exit 1
fi
rm /tmp/sql_error
echo "OK: SQL syntax valid"

# 2. Check for hardcoded IDs (anti-pattern)
echo "2. Checking for hardcoded IDs..."
if grep -q "VALUES ([0-9]" "$SQL_FILE"; then
    echo "WARNING: Hardcoded IDs detected"
    echo "  Use: (SELECT id FROM entities WHERE key = 'name')"
    echo "  Not: VALUES (1, ...)"
else
    echo "OK: No hardcoded IDs"
fi

# 3. Check prerequisite types
echo "3. Checking prerequisite types..."
VALID_TYPES="sp_cost branch_progression tier_requirement scenario_clear hidden_threshold exp_bank_saturation rank_requirement cross_entity_skill cross_entity_node"
PREREQ_TYPES=$(grep -o "prereq_type = '[^']*'" "$SQL_FILE" | cut -d"'" -f2 | sort -u)

ERROR=0
for type in $PREREQ_TYPES; do
    if [[ ! " $VALID_TYPES " =~ " $type " ]]; then
        echo "ERROR: Invalid prerequisite type: '$type'"
        echo "  Valid types: $VALID_TYPES"
        ERROR=1
    fi
done

if [ $ERROR -eq 0 ]; then
    echo "OK: Prerequisite types valid"
fi

# 4. Check node key format
echo "4. Checking node key format..."
NODE_KEYS=$(grep -o "node_key = '[^']*'" "$SQL_FILE" | cut -d"'" -f2)

ERROR=0
for key in $NODE_KEYS; do
    if [[ ! "$key" =~ ^[a-z_]+\.[a-z_]+$ ]]; then
        echo "WARNING: Node key '$key' may not follow pattern: entity.skill"
        ERROR=1
    fi
done

if [ $ERROR -eq 0 ]; then
    echo "OK: Node key format valid"
fi

echo "========================================="
echo "Validation complete."
echo ""
echo "Next: sqlite3 $DB_FILE '.read $SQL_FILE'"