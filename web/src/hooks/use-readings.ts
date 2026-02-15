import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/lib/api";
import type { ReadingResponse, ReadingsQuery } from "@/types";

export function useReadings(params?: ReadingsQuery) {
  const searchParams = new URLSearchParams();
  if (params?.brewId) searchParams.set("brew_id", params.brewId);
  if (params?.hydrometerId) searchParams.set("hydrometer_id", params.hydrometerId);
  if (params?.since) searchParams.set("since", params.since);
  if (params?.until) searchParams.set("until", params.until);
  if (params?.limit) searchParams.set("limit", String(params.limit));

  const query = searchParams.toString();
  const path = `/readings${query ? `?${query}` : ""}`;

  return useQuery<ReadingResponse[]>({
    queryKey: ["readings", params],
    queryFn: () => apiGet<ReadingResponse[]>(path),
  });
}
