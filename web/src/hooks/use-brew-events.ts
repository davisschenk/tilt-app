import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete } from "@/lib/api";
import type {
  BrewEventResponse,
  CreateBrewEvent,
  UpdateBrewEvent,
} from "@/types";

export function useBrewEvents(brewId: string) {
  return useQuery<BrewEventResponse[]>({
    queryKey: ["brew-events", brewId],
    queryFn: () => apiGet<BrewEventResponse[]>(`/brews/${brewId}/events`),
    enabled: !!brewId,
  });
}

export function useCreateBrewEvent(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreateBrewEvent) =>
      apiPost<BrewEventResponse>(`/brews/${brewId}/events`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brew-events", brewId] });
      queryClient.invalidateQueries({ queryKey: ["nutrient-schedule", brewId] });
      queryClient.invalidateQueries({ queryKey: ["brews", brewId] });
    },
  });
}

export function useUpdateBrewEvent(brewId: string, eventId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: UpdateBrewEvent) =>
      apiPut<BrewEventResponse>(`/brews/${brewId}/events/${eventId}`, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brew-events", brewId] });
    },
  });
}

export function useDeleteBrewEvent(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (eventId: string) =>
      apiDelete(`/brews/${brewId}/events/${eventId}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["brew-events", brewId] });
    },
  });
}
