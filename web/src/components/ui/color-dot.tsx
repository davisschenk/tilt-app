import { getColorHex } from "@/lib/tilt-colors";

interface ColorDotProps {
  color: string;
  size?: "sm" | "md" | "lg";
  className?: string;
}

const SIZES = {
  sm: "h-3 w-3",
  md: "h-4 w-4",
  lg: "h-6 w-6",
};

export default function ColorDot({ color, size = "sm", className = "" }: ColorDotProps) {
  return (
    <span
      className={`inline-block rounded-full ${SIZES[size]} ${className}`}
      style={{ backgroundColor: getColorHex(color) }}
    />
  );
}
