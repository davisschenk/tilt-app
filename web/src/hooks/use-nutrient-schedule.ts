import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPut } from "@/lib/api";
import type { NutrientScheduleResponse, UpdateBrew, BrewResponse } from "@/types";

export function useNutrientSchedule(brew: Pick<BrewResponse, "id" | "og" | "targetFg" | "batchSizeGallons" | "pitchTime">) {
  const ready =
    brew.og != null &&
    brew.targetFg != null &&
    brew.batchSizeGallons != null &&
    brew.pitchTime != null;
  return useQuery<NutrientScheduleResponse>({
    queryKey: ["nutrient-schedule", brew.id],
    queryFn: () => apiGet<NutrientScheduleResponse>(`/brews/${brew.id}/nutrient-schedule`),
    enabled: ready,
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
