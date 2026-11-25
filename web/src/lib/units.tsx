import { useMemo } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPut } from "@/lib/api";
import { useAuth } from "@/lib/auth-context";
import { unitTools, defaultUnits, type Preferences } from "./measurement";
export {
  unitTools,
  defaultUnits,
  sgToPlato,
  platoToSg,
  measurementConverters,
} from "./measurement";
export type { Preferences, UnitTools, MeasurementKind } from "./measurement";

export function useUnits() {
  const { user } = useAuth();
  const client = useQueryClient();
  const key = ["preferences", user?.userSub];
  const query = useQuery({
    queryKey: key,
    queryFn: () => apiGet<Preferences>("/account/preferences"),
    enabled: !!user,
  });
  const save = useMutation({
    mutationFn: (p: Preferences) =>
      apiPut<Preferences>("/account/preferences", p),
    onSuccess: (p) => client.setQueryData(key, p),
  });
  // Stable while the preferences are unchanged, so callers can use it in memo deps.
  const tools = useMemo(
    () => unitTools(query.data ?? defaultUnits),
    [query.data],
  );
  return useMemo(
    () => ({ ...tools, save, isLoading: query.isLoading }),
    [tools, save, query.isLoading],
  );
}
