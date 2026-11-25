import { useEffect, useState } from "react";

/**
 * Current time in epoch ms, captured on mount and refreshed every `intervalMs`.
 * Use instead of calling `Date.now()` during render, which is impure.
 */
export function useNow(intervalMs = 60_000): number {
  const [now, setNow] = useState(() => Date.now());

  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(id);
  }, [intervalMs]);

  return now;
}
