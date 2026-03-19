import { useEffect, useState } from "react";
import { useTheme } from "@/components/theme-provider";
import { resolveColor, resolveFont } from "@/lib/chart-theme";

export interface EChartsTheme {
  textColor: string;
  mutedColor: string;
  borderColor: string;
  bgColor: string;
  popoverBg: string;
  popoverFg: string;
  gridColor: string;
  fontFamily: string;
  isDark: boolean;
}

export function useEChartsTheme(): EChartsTheme {
  const { theme } = useTheme();
  const [t, setT] = useState<EChartsTheme>(buildTheme());

  useEffect(() => {
    const id = requestAnimationFrame(() => setT(buildTheme()));
    return () => cancelAnimationFrame(id);
  }, [theme]);

  return t;
}

function buildTheme(): EChartsTheme {
  return {
    textColor: resolveColor("--foreground"),
    mutedColor: resolveColor("--muted-foreground"),
    borderColor: resolveColor("--border"),
    bgColor: resolveColor("--background"),
    popoverBg: resolveColor("--popover"),
    popoverFg: resolveColor("--popover-foreground"),
    gridColor: "rgba(128,128,128,0.1)",
    fontFamily: resolveFont(),
    isDark: document.documentElement.classList.contains("dark"),
  };
}
