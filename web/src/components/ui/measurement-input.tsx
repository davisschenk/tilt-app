import { useState } from "react";
import { Input } from "./input";
import {
  useUnits,
  measurementConverters,
  defaultUnits,
  type MeasurementKind,
} from "@/lib/units";
import type { ComponentProps } from "react";

type Props = Omit<ComponentProps<typeof Input>, "value" | "onChange"> & {
  value: string;
  onChange: (e: { target: { value: string } }) => void;
  measurement: MeasurementKind;
};

/** Edits a canonical (°F / SG) value in the user's preferred unit. */
export function MeasurementInput({
  value,
  onChange,
  measurement,
  ...props
}: Props) {
  const { units } = useUnits();
  const converter = measurementConverters(units)[measurement];
  // Inputs keep at least two decimals so a typed temperature round-trips.
  const decimals = Math.max(converter.decimals, 2);
  const display =
    value === ""
      ? ""
      : String(Number(converter.toDisplay(Number(value)).toFixed(decimals)));

  // Placeholders are written in canonical units, so only show them when this
  // input's unit is the default one.
  const isGravity = measurement === "gravity" || measurement === "gravityOffset";
  const usesDefaultUnit = isGravity
    ? units.gravity === defaultUnits.gravity
    : units.temperature === defaultUnits.temperature;

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
      min={props.min == null ? undefined : converter.toDisplay(Number(props.min))}
      max={props.max == null ? undefined : converter.toDisplay(Number(props.max))}
      step="any"
      placeholder={usesDefaultUnit ? props.placeholder : undefined}
      onChange={(e) => {
        const canonical =
          e.target.value === ""
            ? ""
            : String(converter.toCanonical(Number(e.target.value)));
        setEdit({ draft: e.target.value, canonical, signature });
        onChange({ target: { value: canonical } });
      }}
    />
  );
}
