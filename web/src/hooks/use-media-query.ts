import { useCallback, useMemo, useSyncExternalStore } from "react";

/** Subscribes to a CSS media query and returns whether it currently matches. */
export function useMediaQuery(query: string): boolean {
  const mql = useMemo(() => window.matchMedia(query), [query]);
  const subscribe = useCallback(
    (onChange: () => void) => {
      mql.addEventListener("change", onChange);
      return () => mql.removeEventListener("change", onChange);
    },
    [mql],
  );
  return useSyncExternalStore(subscribe, () => mql.matches);
}

/** True below Tailwind's `md` breakpoint (48rem). */
export function useIsMobile(): boolean {
  return useMediaQuery("(max-width: 767.98px)");
}
