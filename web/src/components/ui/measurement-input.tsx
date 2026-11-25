import { useState } from "react";
import { Input } from "./input";
import { useUnits, sgToPlato, platoToSg } from "@/lib/units";
import type { ComponentProps } from "react";
type Props = Omit<ComponentProps<typeof Input>, "value" | "onChange"> & {
  value: string;
  onChange: (e: { target: { value: string } }) => void;
  measurement: "gravity" | "temperature" | "gravityOffset" | "tempOffset";
};
export function MeasurementInput({
  value,
  onChange,
  measurement,
  ...props
}: Props) {
  const { units } = useUnits();
  const toDisplay = (v: number) =>
    measurement === "gravity"
      ? units.gravity === "P"
        ? sgToPlato(v)
        : v
      : measurement === "gravityOffset"
        ? units.gravity === "P"
          ? sgToPlato(1 + v) - sgToPlato(1)
          : v
        : units.temperature === "C"
          ? ((measurement === "tempOffset" ? v : v - 32) * 5) / 9
          : v;
  const toCanonical = (v: number) =>
    measurement === "gravity"
      ? units.gravity === "P"
        ? platoToSg(v)
        : v
      : measurement === "gravityOffset"
        ? units.gravity === "P"
          ? platoToSg(v + sgToPlato(1)) - 1
          : v
        : units.temperature === "C"
          ? (v * 9) / 5 + (measurement === "tempOffset" ? 0 : 32)
          : v;
  const display =
    value === ""
      ? ""
      : String(
          Number(
            toDisplay(Number(value)).toFixed(
              measurement.startsWith("gravity")
                ? units.gravity === "P"
                  ? 2
                  : measurement === "gravityOffset"
                    ? 4
                    : 3
                : 2,
            ),
          ),
        );
  const signature = `${units.temperature}:${units.gravity}:${measurement}`;
  const [edit, setEdit] = useState({
    draft: display,
    canonical: value,
    signature,
  });
  const draft =
    edit.canonical === value && edit.signature === signature
      ? edit.draft
      : display;
  return (
    <Input
      {...props}
      value={draft}
      min={props.min == null ? undefined : toDisplay(Number(props.min))}
      max={props.max == null ? undefined : toDisplay(Number(props.max))}
      step="any"
      placeholder={units.temperature === "F" && units.gravity === "SG" ? props.placeholder : undefined}
      onChange={(e) => {
        const canonical =
          e.target.value === ""
            ? ""
            : String(toCanonical(Number(e.target.value)));
        setEdit({ draft: e.target.value, canonical, signature });
        onChange({ target: { value: canonical } });
      }}
    />
  );
}
