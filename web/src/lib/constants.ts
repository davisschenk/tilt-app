export const OFFLINE_THRESHOLD_MINUTES = 45;

/** True when a reading at `recordedAt` is older than the offline threshold, relative to `now` (epoch ms). */
export function isStale(recordedAt: string, now: number): boolean {
  return now - new Date(recordedAt).getTime() > OFFLINE_THRESHOLD_MINUTES * 60 * 1000;
}
