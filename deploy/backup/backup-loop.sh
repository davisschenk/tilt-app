#!/bin/sh
# Runs backup.sh once a day at BACKUP_HOUR_UTC (default 03:00 UTC). A failed run
# is logged and retried the next day; the loop itself never exits.
set -u
HOUR="${BACKUP_HOUR_UTC:-3}"
DIR="$(dirname "$0")"

while true; do
  now="$(date -u +%s)"
  target=$(( now - now % 86400 + HOUR * 3600 ))
  [ "$target" -le "$now" ] && target=$(( target + 86400 ))
  echo "backup: next run in $(( (target - now) / 60 )) minutes"
  sleep $(( target - now ))
  sh "$DIR/backup.sh" || echo "backup: FAILED (will retry tomorrow)" >&2
done
