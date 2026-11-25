import { useCallback, useSyncExternalStore } from "react";

/** Subscribes to a CSS media query and returns whether it currently matches. */
export function useMediaQuery(query: string): boolean {
  const subscribe = useCallback(
    (onChange: () => void) => {
      const mql = window.matchMedia(query);
      mql.addEventListener("change", onChange);
      return () => mql.removeEventListener("change", onChange);
    },
    [query],
  );
  return useSyncExternalStore(subscribe, () => window.matchMedia(query).matches);
}

/**
 * True below Tailwind's `md` breakpoint (48rem). Use this instead of pairing
 * `md:hidden` / `hidden md:block` when the hidden branch would mount something
 * that measures itself (e.g. an ECharts canvas sizes to 0×0 under display:none).
 */
export function useIsMobile(): boolean {
  return useMediaQuery("(max-width: 767.98px)");
}
