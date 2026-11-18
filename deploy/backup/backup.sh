#!/bin/sh
# One backup run: a Postgres dump plus a tarball of uploaded photos, verified,
# rotated, and optionally copied off-host with rclone.
#
# Needs PGHOST, PGUSER, PGDATABASE and PGPASSWORD (standard libpq variables).
set -eu

: "${PGHOST:?PGHOST is required}" "${PGUSER:?PGUSER is required}" "${PGDATABASE:?PGDATABASE is required}"
BACKUP_DIR="${BACKUP_DIR:-/backups}"
UPLOAD_DIR="${UPLOAD_DIR:-/uploads}"
KEEP_DAYS="${BACKUP_KEEP_DAYS:-14}"

ts="$(date -u +%Y%m%dT%H%M%SZ)"
db="$BACKUP_DIR/db-$ts.dump"
files="$BACKUP_DIR/uploads-$ts.tar.gz"
mkdir -p "$BACKUP_DIR"

echo "backup: dumping $PGDATABASE@$PGHOST to $db"
# Write to a .partial name first so a crash never leaves a truncated file that
# looks like a good backup.
pg_dump --format=custom --no-owner --file="$db.partial"
# A dump pg_restore cannot read is not a backup.
pg_restore --list "$db.partial" >/dev/null
mv "$db.partial" "$db"

if [ -d "$UPLOAD_DIR" ]; then
  echo "backup: archiving $UPLOAD_DIR to $files"
  tar -C "$UPLOAD_DIR" -czf "$files.partial" .
  tar -tzf "$files.partial" >/dev/null
  mv "$files.partial" "$files"
else
  echo "backup: no $UPLOAD_DIR, skipping uploads"
fi

# Rotate local copies. Leftover .partial files from a crashed run go too.
find "$BACKUP_DIR" -maxdepth 1 \( -name 'db-*.dump' -o -name 'uploads-*.tar.gz' \) -mtime "+$KEEP_DAYS" -delete
find "$BACKUP_DIR" -maxdepth 1 -name '*.partial' -mmin +60 -delete

if [ -n "${BACKUP_RCLONE_REMOTE:-}" ]; then
  command -v rclone >/dev/null 2>&1 || apk add --no-cache rclone >/dev/null
  echo "backup: copying to $BACKUP_RCLONE_REMOTE"
  rclone copy "$db" "$BACKUP_RCLONE_REMOTE"
  [ -f "$files" ] && rclone copy "$files" "$BACKUP_RCLONE_REMOTE"
  # Same retention off-host, so the remote does not grow forever.
  rclone delete "$BACKUP_RCLONE_REMOTE" --min-age "${KEEP_DAYS}d" --include 'db-*.dump' --include 'uploads-*.tar.gz'
fi

echo "backup: done ($(du -h "$db" | cut -f1) database)"
