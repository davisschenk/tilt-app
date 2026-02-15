import type { TiltColor } from "@/types";

export interface TiltColorInfo {
  hex: string;
  displayName: string;
  bgLight: string;
}

export const TILT_COLOR_MAP: Record<TiltColor, TiltColorInfo> = {
  Red: { hex: "#E03131", displayName: "Red", bgLight: "#FFF5F5" },
  Green: { hex: "#2F9E44", displayName: "Green", bgLight: "#EBFBEE" },
  Black: { hex: "#495057", displayName: "Black", bgLight: "#F8F9FA" },
  Purple: { hex: "#7048E8", displayName: "Purple", bgLight: "#F3F0FF" },
  Orange: { hex: "#E8590C", displayName: "Orange", bgLight: "#FFF4E6" },
  Blue: { hex: "#1971C2", displayName: "Blue", bgLight: "#E7F5FF" },
  Yellow: { hex: "#F08C00", displayName: "Yellow", bgLight: "#FFF9DB" },
  Pink: { hex: "#D6336C", displayName: "Pink", bgLight: "#FFF0F6" },
};

export const ALL_TILT_COLORS: TiltColor[] = [
  "Red",
  "Green",
  "Black",
  "Purple",
  "Orange",
  "Blue",
  "Yellow",
  "Pink",
];

export function getColorHex(color: string): string {
  return (TILT_COLOR_MAP as Record<string, TiltColorInfo>)[color]?.hex ?? "#868E96";
}
