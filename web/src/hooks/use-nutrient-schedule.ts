import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPut } from "@/lib/api";
import type { NutrientScheduleResponse, UpdateBrew, BrewResponse } from "@/types";

export function useNutrientSchedule(brewId: string) {
  return useQuery<NutrientScheduleResponse>({
    queryKey: ["nutrient-schedule", brewId],
    queryFn: () => apiGet<NutrientScheduleResponse>(`/brews/${brewId}/nutrient-schedule`),
    enabled: !!brewId,
    retry: false,
  });
}

export function useUpdateBrewNutrientSetup(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateBrew) => apiPut<BrewResponse>(`/brews/${brewId}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brews", brewId] });
      queryClient.invalidateQueries({ queryKey: ["nutrient-schedule", brewId] });
    },
  });
}
