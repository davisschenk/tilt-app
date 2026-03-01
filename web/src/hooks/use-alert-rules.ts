import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete } from "@/lib/api";
import type {
  AlertRuleResponse,
  CreateAlertRule,
  UpdateAlertRule,
} from "@/types";

export function useAlertRules(brewId?: string, hydrometerId?: string) {
  const params = new URLSearchParams();
  if (brewId) params.set("brew_id", brewId);
  if (hydrometerId) params.set("hydrometer_id", hydrometerId);
  const qs = params.toString();
  const path = qs ? `/alert-rules?${qs}` : "/alert-rules";

  return useQuery<AlertRuleResponse[]>({
    queryKey: ["alert-rules", brewId ?? "", hydrometerId ?? ""],
    queryFn: () => apiGet<AlertRuleResponse[]>(path),
  });
}

export function useAlertRule(id: string) {
  return useQuery<AlertRuleResponse>({
    queryKey: ["alert-rules", id],
    queryFn: () => apiGet<AlertRuleResponse>(`/alert-rules/${id}`),
    enabled: !!id,
  });
}

export function useCreateAlertRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateAlertRule) =>
      apiPost<AlertRuleResponse>("/alert-rules", data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
    },
  });
}

export function useUpdateAlertRule(id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateAlertRule) =>
      apiPut<AlertRuleResponse>(`/alert-rules/${id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
    },
  });
}

export function useDeleteAlertRule() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDelete(`/alert-rules/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-rules"] });
    },
  });
}
