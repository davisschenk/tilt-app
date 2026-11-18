import { useQuery } from "@tanstack/react-query";
import { apiGet } from "@/lib/api";

export interface ApiKeySummary {
  id: string;
  name: string;
  prefix: string;
  createdBy: string;
  lastUsedAt: string | null;
  expiresAt: string | null;
  createdAt: string;
}

export function useApiKeys() {
  return useQuery<ApiKeySummary[]>({
    queryKey: ["api-keys"],
    queryFn: () => apiGet<ApiKeySummary[]>("/api-keys"),
  });
}
