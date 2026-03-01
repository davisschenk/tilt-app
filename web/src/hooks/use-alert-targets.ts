import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete } from "@/lib/api";
import type {
  AlertTargetResponse,
  CreateAlertTarget,
  UpdateAlertTarget,
  TestFireResult,
} from "@/types";

export function useAlertTargets() {
  return useQuery<AlertTargetResponse[]>({
    queryKey: ["alert-targets"],
    queryFn: () => apiGet<AlertTargetResponse[]>("/alert-targets"),
  });
}

export function useAlertTarget(id: string) {
  return useQuery<AlertTargetResponse>({
    queryKey: ["alert-targets", id],
    queryFn: () => apiGet<AlertTargetResponse>(`/alert-targets/${id}`),
    enabled: !!id,
  });
}

export function useCreateAlertTarget() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateAlertTarget) =>
      apiPost<AlertTargetResponse>("/alert-targets", data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-targets"] });
    },
  });
}

export function useUpdateAlertTarget(id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateAlertTarget) =>
      apiPut<AlertTargetResponse>(`/alert-targets/${id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-targets"] });
    },
  });
}

export function useDeleteAlertTarget() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDelete(`/alert-targets/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["alert-targets"] });
    },
  });
}

export function useTestFireTarget() {
  return useMutation({
    mutationFn: (id: string) =>
      apiPost<TestFireResult>(`/alert-targets/${id}/test`, {}),
  });
}
