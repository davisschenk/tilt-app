import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPut } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { unitTools, defaultUnits, type Preferences } from "./measurement";
export { unitTools, defaultUnits, sgToPlato, platoToSg } from "./measurement";
export type { Preferences } from "./measurement";
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
  return {
    ...unitTools(query.data ?? defaultUnits),
    save,
    isLoading: query.isLoading,
  };
}
