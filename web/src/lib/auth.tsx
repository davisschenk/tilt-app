import { useCallback, type ReactNode } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { apiGet, apiPost, ApiError } from "@/lib/api";
import { AuthContext, type AuthUser } from "@/lib/auth-context";

const API_BASE_URL = import.meta.env.VITE_API_URL ?? "/api/v1";

const AUTH_ME_KEY = ["auth", "me"] as const;

/** The session user, or `null` when signed out (401). Other errors throw, so React Query
 *  keeps the last known user on transient failures. */
async function fetchSessionUser(): Promise<AuthUser | null> {
  try {
    return await apiGet<AuthUser>("/auth/me");
  } catch (err) {
    if (err instanceof ApiError && err.status === 401) return null;
    throw err;
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();
  const { data, isPending, refetch } = useQuery({
    queryKey: AUTH_ME_KEY,
    queryFn: fetchSessionUser,
    retry: false,
    // Fetched once on mount; `refresh()` re-checks explicitly.
    staleTime: Infinity,
  });
  const user = data ?? null;

  const refresh = useCallback(async () => {
    await refetch();
  }, [refetch]);

  const login = useCallback(() => {
    window.location.href = `${API_BASE_URL}/auth/login`;
  }, []);

  const logout = useCallback(async () => {
    try {
      await apiPost("/auth/logout", {});
    } catch {
      // ignore errors on logout
    }
    queryClient.setQueryData(AUTH_ME_KEY, null);
    window.location.href = "/";
  }, [queryClient]);

  return (
    <AuthContext.Provider
      value={{
        user,
        isLoading: isPending,
        isAuthenticated: user !== null,
        login,
        logout,
        refresh,
      }}
    >
      {children}
    </AuthContext.Provider>
  );
}
