#!/bin/bash
# validate_schema.sh — Compare two SQLite databases and report structural differences
# Usage: ./validate_schema.sh <db1.sqlite3> <db2.sqlite3>

set -euo pipefail

if [ $# -ne 2 ]; then
    echo "Usage: $0 <db1.sqlite3> <db2.sqlite3>"
    exit 1
fi

DB1="$1"
DB2="$2"

if [ ! -f "$DB1" ]; then
    echo "Error: $DB1 not found"
    exit 1
fi

if [ ! -f "$DB2" ]; then
    echo "Error: $DB2 not found"
    exit 1
fi

# Get table list from each DB
get_tables() {
    sqlite3 "$1" ".tables" | tr -s ' ' '\n' | sort
}

# Get column info for a table
get_columns() {
    sqlite3 "$1" "PRAGMA table_info($2);" | awk -F'|' '{print $2"|"$3"|"$4"|"$5"|"$6}'
}

# Get indexes for a table
get_indexes() {
    sqlite3 "$1" "PRAGMA index_list($2);" | awk -F'|' '{print $2"|"$3"|"$4}' | sort
}

# Get index columns
get_index_columns() {
    local db="$1"
    local idx="$2"
    sqlite3 "$db" "PRAGMA index_info($idx);" | awk -F'|' '{print $3}' | sort | tr '\n' ',' | sed 's/,$//'
}

# Compute schema checksum for a table
compute_checksum() {
    local db="$1"
    local table="$2"
    
    {
        echo "$table"
        get_columns "$db" "$table" | while IFS='|' read -r name type notnull dflt pk; do
            echo "$name|$type|$notnull|$dflt|$pk"
        done
        get_indexes "$db" "$table" | while IFS='|' read -r name unique origin partial; do
            cols=$(get_index_columns "$db" "$name")
            echo "INDEX:$name:$unique:$cols"
        done
    } | sha256sum | cut -d' ' -f1
}

echo "Schema comparison: $DB1 vs $DB2"
echo ""

# Get tables from each DB
TABLES1=$(get_tables "$DB1")
TABLES2=$(get_tables "$DB2")

# Find added/removed tables
ADDED=$(comm -13 <(echo "$TABLES1") <(echo "$TABLES2"))
REMOVED=$(comm -23 <(echo "$TABLES1") <(echo "$TABLES2"))
COMMON=$(comm -12 <(echo "$TABLES1") <(echo "$TABLES2"))

if [ -n "$ADDED" ]; then
    echo "Tables added:"
    echo "$ADDED" | sed 's/^/  + /'
    echo ""
fi

if [ -n "$REMOVED" ]; then
    echo "Tables removed:"
    echo "$REMOVED" | sed 's/^/  - /'
    echo ""
fi

# Compare common tables
CHANGES=0
for table in $COMMON; do
    CHECKSUM1=$(compute_checksum "$DB1" "$table")
    CHECKSUM2=$(compute_checksum "$DB2" "$table")
    
    if [ "$CHECKSUM1" != "$CHECKSUM2" ]; then
        echo "Table: $table"
        CHANGES=$((CHANGES + 1))
        
        # Compare columns
        COLS1=$(get_columns "$DB1" "$table")
        COLS2=$(get_columns "$DB2" "$table")
        
        ADDED_COLS=$(comm -13 <(echo "$COLS1") <(echo "$COLS2"))
        REMOVED_COLS=$(comm -23 <(echo "$COLS1") <(echo "$COLS2"))
        
        if [ -n "$ADDED_COLS" ]; then
            echo "$ADDED_COLS" | sed 's/^/    [ADDED] column: /'
        fi
        
        if [ -n "$REMOVED_COLS" ]; then
            echo "$REMOVED_COLS" | sed 's/^/    [REMOVED] column: /'
        fi
        
        # Compare indexes
        IDX1=$(get_indexes "$DB1" "$table")
        IDX2=$(get_indexes "$DB2" "$table")
        
        ADDED_IDX=$(comm -13 <(echo "$IDX1") <(echo "$IDX2"))
        REMOVED_IDX=$(comm -23 <(echo "$IDX1") <(echo "$IDX2"))
        
        if [ -n "$ADDED_IDX" ]; then
            echo "$ADDED_IDX" | sed 's/^/    [ADDED] index: /'
        fi
        
        if [ -n "$REMOVED_IDX" ]; then
            echo "$REMOVED_IDX" | sed 's/^/    [REMOVED] index: /'
        fi
        
        echo ""
    fi
done

if [ $CHANGES -eq 0 ] && [ -z "$ADDED" ] && [ -z "$REMOVED" ]; then
    echo "Schema identical"
fi
