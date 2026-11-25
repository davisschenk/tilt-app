import { useEffect, useState } from "react";

/**
 * Tracks an element's content width (rounded to whole pixels) with a ResizeObserver.
 *
 * Returns a callback ref rather than reading a ref object in a mount-only effect:
 * charts render a skeleton while loading, so their container mounts *after* the
 * component does, and a `useEffect(..., [])` would never see it. The observer's
 * initial notification supplies the first real width.
 */
export function useElementWidth<T extends HTMLElement>(initialWidth: number) {
  const [element, setElement] = useState<T | null>(null);
  const [width, setWidth] = useState(initialWidth);

  useEffect(() => {
    if (!element) return;
    const ro = new ResizeObserver((entries) => {
      // React bails out of the re-render when the rounded width is unchanged.
      setWidth(Math.round(entries[0]?.contentRect.width ?? element.clientWidth));
    });
    ro.observe(element);
    return () => ro.disconnect();
  }, [element]);

  return { ref: setElement, width };
}
