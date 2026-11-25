import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/lib/api";
import type { ScannerResponse } from "@/types";

/** Scanner heartbeats; polled so the online/stale status stays current. */
export function useScanners() {
  return useQuery<ScannerResponse[]>({
    queryKey: ["scanners"],
    queryFn: () => apiGet<ScannerResponse[]>("/scanners"),
    refetchInterval: 30_000,
  });
}
