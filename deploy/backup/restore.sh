#!/bin/sh
# Restore a backup made by backup.sh. DESTRUCTIVE: replaces the contents of the
# target database (and merges the photo archive into UPLOAD_DIR).
#
#   restore.sh /backups/db-20261118T030000Z.dump [/backups/uploads-20261118T030000Z.tar.gz]
#
# Needs PGHOST, PGUSER, PGDATABASE, PGPASSWORD. Stop the server first.
# Set FORCE=1 to skip the confirmation prompt.
set -eu

DUMP="${1:?usage: restore.sh <db-dump> [uploads-archive]}"
FILES="${2:-}"
: "${PGHOST:?}" "${PGUSER:?}" "${PGDATABASE:?}"
UPLOAD_DIR="${UPLOAD_DIR:-/uploads}"

[ -f "$DUMP" ] || { echo "restore: $DUMP not found" >&2; exit 1; }
pg_restore --list "$DUMP" >/dev/null || { echo "restore: $DUMP is not a readable dump" >&2; exit 1; }

if [ "${FORCE:-}" != "1" ]; then
  printf 'This REPLACES database "%s" on %s with %s. Type the database name to continue: ' "$PGDATABASE" "$PGHOST" "$DUMP"
  read -r answer
  [ "$answer" = "$PGDATABASE" ] || { echo "restore: aborted"; exit 1; }
fi

echo "restore: loading $DUMP"
pg_restore --clean --if-exists --no-owner --single-transaction --dbname="$PGDATABASE" "$DUMP"

if [ -n "$FILES" ]; then
  [ -f "$FILES" ] || { echo "restore: $FILES not found" >&2; exit 1; }
  echo "restore: unpacking $FILES into $UPLOAD_DIR"
  mkdir -p "$UPLOAD_DIR"
  tar -C "$UPLOAD_DIR" -xzf "$FILES"
fi
echo "restore: done"
