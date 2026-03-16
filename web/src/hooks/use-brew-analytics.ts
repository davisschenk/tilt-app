import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/lib/api";
import type { BrewAnalytics } from "@/types";

export function useBrewAnalytics(brewId: string) {
  return useQuery<BrewAnalytics>({
    queryKey: ["brew-analytics", brewId],
    queryFn: () => apiGet<BrewAnalytics>(`/brews/${brewId}/analytics`),
    enabled: !!brewId,
    refetchInterval: 60_000,
  });
}
