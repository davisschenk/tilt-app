import { useState, useEffect, useCallback, type ReactNode } from "react";
import { apiGet, apiPost, ApiError } from "@/lib/api";
import { AuthContext, type AuthUser } from "@/lib/auth-context";

const API_BASE_URL = import.meta.env.VITE_API_URL ?? "/api/v1";

/** Fetch the session user: the user, `null` when signed out (401), or `undefined` on other errors. */
async function fetchSessionUser(): Promise<AuthUser | null | undefined> {
  try {
    return await apiGet<AuthUser>("/auth/me");
  } catch (err) {
    return err instanceof ApiError && err.status === 401 ? null : undefined;
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const applySessionUser = useCallback((me: AuthUser | null | undefined) => {
    // Transient (non-401) errors keep whatever user we already had.
    if (me !== undefined) setUser(me);
    setIsLoading(false);
  }, []);

  const refresh = useCallback(async () => {
    applySessionUser(await fetchSessionUser());
  }, [applySessionUser]);

  useEffect(() => {
    let active = true;
    fetchSessionUser().then((me) => {
      if (active) applySessionUser(me);
    });
    return () => {
      active = false;
    };
  }, [applySessionUser]);

  const login = useCallback(() => {
    window.location.href = `${API_BASE_URL}/auth/login`;
  }, []);

  const logout = useCallback(async () => {
    try {
      await apiPost("/auth/logout", {});
    } catch {
      // ignore errors on logout
    }
    setUser(null);
    window.location.href = "/";
  }, []);

  return (
    <AuthContext.Provider
      value={{
        user,
        isLoading,
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
