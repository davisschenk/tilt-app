import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiDelete } from "@/lib/api";
import type {
  NutrientScheduleResponse,
  CreateNutrientSchedule,
  NutrientCalculateRequest,
  NutrientCalculateResponse,
} from "@/types";

export function useNutrientSchedule(brewId: string) {
  return useQuery<NutrientScheduleResponse | null>({
    queryKey: ["nutrient-schedule", brewId],
    queryFn: async () => {
      try {
        return await apiGet<NutrientScheduleResponse>(
          `/brews/${brewId}/nutrient-schedule`
        );
      } catch {
        return null;
      }
    },
    enabled: !!brewId,
  });
}

export function useCreateNutrientSchedule(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateNutrientSchedule) =>
      apiPost<NutrientScheduleResponse>(
        `/brews/${brewId}/nutrient-schedule`,
        data
      ),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["nutrient-schedule", brewId],
      });
      queryClient.invalidateQueries({ queryKey: ["brew-events", brewId] });
    },
  });
}

export function useDeleteNutrientSchedule(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => apiDelete(`/brews/${brewId}/nutrient-schedule`),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ["nutrient-schedule", brewId],
      });
    },
  });
}

export function useNutrientCalculate() {
  return useMutation({
    mutationFn: (data: NutrientCalculateRequest) =>
      apiPost<NutrientCalculateResponse>("/nutrients/calculate", data),
  });
}
