import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete } from "@/lib/api";
import type {
  HydrometerResponse,
  CreateHydrometer,
  UpdateHydrometer,
} from "@/types";

export function useHydrometers() {
  return useQuery<HydrometerResponse[]>({
    queryKey: ["hydrometers"],
    queryFn: () => apiGet<HydrometerResponse[]>("/hydrometers"),
  });
}

export function useHydrometer(id: string) {
  return useQuery<HydrometerResponse>({
    queryKey: ["hydrometers", id],
    queryFn: () => apiGet<HydrometerResponse>(`/hydrometers/${id}`),
    enabled: !!id,
  });
}

export function useCreateHydrometer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateHydrometer) =>
      apiPost<HydrometerResponse>("/hydrometers", data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["hydrometers"] });
    },
  });
}

export function useUpdateHydrometer(id: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateHydrometer) =>
      apiPut<HydrometerResponse>(`/hydrometers/${id}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["hydrometers"] });
    },
  });
}

export function useDeleteHydrometer() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => apiDelete(`/hydrometers/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["hydrometers"] });
    },
  });
}
