import type { AlertMetric, AlertOperator, AlertRuleResponse } from "@/types";
import type { UnitTools } from "./measurement";

const METRIC_LABELS: Record<AlertMetric, string> = {
  gravity: "Gravity",
  temperature_f: "Temperature",
  gravity_plateau: "Gravity Plateau",
};

const OPERATOR_SYMBOLS: Record<AlertOperator, string> = {
  lte: "≤",
  gte: "≥",
  lt: "<",
  gt: ">",
  eq: "=",
  plateau: "plateau",
};

type RuleCondition = Pick<
  AlertRuleResponse,
  "metric" | "operator" | "threshold" | "windowHours"
>;

/** A rule's condition in the viewer's units, e.g. "Gravity ≤ 1.010 SG". */
export function formatCondition(rule: RuleCondition, u: UnitTools): string {
  if (rule.metric === "gravity_plateau") {
    const delta = u.formatNumber("gravityOffset", rule.threshold);
    return `Gravity flat ±${delta} over ${rule.windowHours ?? 24}h`;
  }
  const label =
    rule.metric === "temperature_f"
      ? `${METRIC_LABELS.temperature_f} (${u.temperatureLabel})`
      : (METRIC_LABELS[rule.metric] ?? rule.metric);
  const operator = OPERATOR_SYMBOLS[rule.operator] ?? rule.operator;
  const value =
    rule.metric === "temperature_f"
      ? u.formatTemperature(rule.threshold)
      : u.formatGravity(rule.threshold);
  return `${label} ${operator} ${value}`;
}
