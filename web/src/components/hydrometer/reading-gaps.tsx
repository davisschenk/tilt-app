import { useBrewAnalytics } from "@/hooks/use-brew-analytics";

/** Summary of gaps in a brew's readings (each over 45 minutes). */
export default function ReadingGaps({ brewId }: { brewId: string }) {
  const q = useBrewAnalytics(brewId);
  if (!q.data) return null;
  const minutes = q.data.gaps.reduce((sum, g) => sum + g.durationMinutes, 0);
  return (
    <p className="text-sm">
      Reading gaps: {q.data.gaps.length} ({Math.round(minutes)} min total). Gaps
      exceed 45 minutes.
    </p>
  );
}
