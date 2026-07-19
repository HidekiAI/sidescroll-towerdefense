#!/bin/bash
# generate_checksums.sh — Generate schema_checksums rows for a SQLite database
# Usage: ./generate_checksums.sh <db.sqlite3> <version>

set -euo pipefail

if [ $# -ne 2 ]; then
    echo "Usage: $0 <db.sqlite3> <version>"
    exit 1
fi

DB="$1"
VERSION="$2"

if [ ! -f "$DB" ]; then
    echo "Error: $DB not found"
    exit 1
fi

# Get table list
TABLES=$(sqlite3 "$DB" ".tables" | tr -s ' ' '\n' | sort)

# Get column info for a table
get_columns() {
    sqlite3 "$DB" "PRAGMA table_info($1);" | awk -F'|' '{print $2"|"$3"|"$4"|"$5"|"$6}'
}

# Get indexes for a table
get_indexes() {
    sqlite3 "$DB" "PRAGMA index_list($1);" | awk -F'|' '{print $2"|"$3"|"$4}' | sort
}

# Get index columns
get_index_columns() {
    local idx="$1"
    sqlite3 "$DB" "PRAGMA index_info($idx);" | awk -F'|' '{print $3}' | sort | tr '\n' ',' | sed 's/,$//'
}

# Compute schema checksum for a table
compute_checksum() {
    local table="$1"
    
    {
        echo "$table"
        get_columns "$table" | while IFS='|' read -r name type notnull dflt pk; do
            echo "$name|$type|$notnull|$dflt|$pk"
        done
        get_indexes "$table" | while IFS='|' read -r name unique origin partial; do
            cols=$(get_index_columns "$name")
            echo "INDEX:$name:$unique:$cols"
        done
    } | sha256sum | cut -d' ' -f1
}

echo "-- Schema checksums for $DB (version $VERSION)"
echo "-- Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo ""

# Create version if not exists
VERSION_ID=$(sqlite3 "$DB" "SELECT id FROM schema_version WHERE version = '$VERSION';")
if [ -z "$VERSION_ID" ]; then
    sqlite3 "$DB" "INSERT INTO schema_version (version) VALUES ('$VERSION');"
    VERSION_ID=$(sqlite3 "$DB" "SELECT id FROM schema_version WHERE version = '$VERSION';")
fi

echo "INSERT INTO schema_checksums (version_id, table_name, checksum) VALUES"
FIRST=1
for table in $TABLES; do
    CHECKSUM=$(compute_checksum "$table")
    if [ $FIRST -eq 1 ]; then
        FIRST=0
    else
        echo ","
    fi
    printf "    (%d, '%s', '%s')" "$VERSION_ID" "$table" "$CHECKSUM"
done
echo ";"
