import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete } from "@/lib/api";
import type {
  BrewResponse,
  BrewStatus,
  CreateBrew,
  UpdateBrew,
} from "@/types";

export function useBrews(status?: BrewStatus) {
  const params = status ? `?status=${status}` : "";
  return useQuery<BrewResponse[]>({
    queryKey: ["brews", status],
    queryFn: () => apiGet<BrewResponse[]>(`/brews${params}`),
  });
}

export function useBrew(id: string) {
  return useQuery<BrewResponse>({
    queryKey: ["brews", id],
    queryFn: () => apiGet<BrewResponse>(`/brews/${id}`),
    enabled: !!id,
  });
}

export function useCreateBrew() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateBrew) =>
      apiPost<BrewResponse>("/brews", data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brews"] });
    },
  });
}

export function useUpdateBrew(id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateBrew) =>
      apiPut<BrewResponse>(`/brews/${id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brews"] });
    },
  });
}

export function useDeleteBrew() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDelete(`/brews/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brews"] });
    },
  });
}
