import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, apiPut, apiDelete } from "@/lib/api";
import type {
  BrewReportLink,
  BrewReportSnapshot,
  PublishBrewReport,
} from "@/types";

export function useBrewReports(brewId: string) {
  return useQuery<BrewReportLink[]>({
    queryKey: ["reports", brewId],
    queryFn: () => apiGet<BrewReportLink[]>(`/brews/${brewId}/reports`),
    enabled: !!brewId,
  });
}

/** Create a share link, or refresh an existing one's snapshot when `reportId` is given. */
export function usePublishBrewReport(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({
      options,
      reportId,
    }: {
      options: PublishBrewReport;
      reportId?: string;
    }) =>
      reportId
        ? apiPut<BrewReportLink>(`/brews/${brewId}/reports/${reportId}`, options)
        : apiPost<BrewReportLink>(`/brews/${brewId}/reports`, options),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["reports", brewId] });
    },
  });
}

export function useRevokeBrewReport(brewId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (reportId: string) =>
      apiDelete(`/brews/${brewId}/reports/${reportId}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["reports", brewId] });
    },
  });
}

/** A published report, as anyone with the link sees it. */
export function usePublicBrewReport(token: string | undefined) {
  return useQuery<BrewReportSnapshot>({
    queryKey: ["public-report", token],
    queryFn: () => apiGet<BrewReportSnapshot>(`/reports/${token}`),
    enabled: !!token,
    retry: false,
    staleTime: 0,
    refetchInterval: 30_000,
  });
}
