export type TiltColor =
  | "Red"
  | "Green"
  | "Black"
  | "Purple"
  | "Orange"
  | "Blue"
  | "Yellow"
  | "Pink";

export type BrewStatus = "Active" | "Completed" | "Archived";

export interface TiltReading {
  color: TiltColor;
  temperatureF: number;
  gravity: number;
  rssi: number | null;
  recordedAt: string;
}

export type CreateReadingsBatch = TiltReading[];

export interface CreateBrew {
  name: string;
  hydrometerId: string;
  style?: string | null;
  og?: number | null;
  targetFg?: number | null;
  notes?: string | null;
}

export interface UpdateBrew {
  name?: string | null;
  style?: string | null;
  og?: number | null;
  fg?: number | null;
  targetFg?: number | null;
  abv?: number | null;
  status?: BrewStatus | null;
  notes?: string | null;
  endDate?: string | null;
}

export interface BrewResponse {
  id: string;
  name: string;
  style: string | null;
  og: number | null;
  fg: number | null;
  targetFg: number | null;
  abv: number | null;
  status: BrewStatus;
  startDate: string | null;
  endDate: string | null;
  notes: string | null;
  hydrometerId: string;
  createdAt: string;
  updatedAt: string;
  latestReading: TiltReading | null;
}

export interface CreateHydrometer {
  color: TiltColor;
  name?: string | null;
}

export interface UpdateHydrometer {
  name?: string | null;
  tempOffsetF?: number | null;
  gravityOffset?: number | null;
}

export interface HydrometerResponse {
  id: string;
  color: TiltColor;
  name: string | null;
  tempOffsetF: number;
  gravityOffset: number;
  createdAt: string;
  latestReading: TiltReading | null;
}

export interface ReadingResponse {
  id: string;
  brewId: string | null;
  hydrometerId: string;
  color: TiltColor;
  temperatureF: number;
  gravity: number;
  rssi: number | null;
  recordedAt: string;
  createdAt: string;
}

export interface ReadingsQuery {
  brewId?: string;
  hydrometerId?: string;
  since?: string;
  until?: string;
  limit?: number;
}

export type WebhookFormat = "generic_json" | "discord" | "slack";
export type AlertMetric = "gravity" | "temperature_f";
export type AlertOperator = "lte" | "gte" | "lt" | "gt" | "eq";

export interface AlertTargetResponse {
  id: string;
  name: string;
  url: string;
  format: WebhookFormat;
  secretHeader: string | null;
  enabled: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface CreateAlertTarget {
  name: string;
  url: string;
  format: WebhookFormat;
  secretHeader?: string | null;
  enabled?: boolean;
}

export interface UpdateAlertTarget {
  name?: string | null;
  url?: string | null;
  format?: WebhookFormat | null;
  secretHeader?: string | null;
  enabled?: boolean | null;
}

export interface AlertRuleResponse {
  id: string;
  name: string;
  brewId: string | null;
  hydrometerId: string | null;
  metric: AlertMetric;
  operator: AlertOperator;
  threshold: number;
  alertTargetId: string;
  enabled: boolean;
  cooldownMinutes: number;
  lastTriggeredAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateAlertRule {
  name: string;
  metric: AlertMetric;
  operator: AlertOperator;
  threshold: number;
  alertTargetId: string;
  brewId?: string | null;
  hydrometerId?: string | null;
  cooldownMinutes?: number;
  enabled?: boolean;
}

export interface UpdateAlertRule {
  name?: string | null;
  metric?: AlertMetric | null;
  operator?: AlertOperator | null;
  threshold?: number | null;
  alertTargetId?: string | null;
  brewId?: string | null;
  hydrometerId?: string | null;
  cooldownMinutes?: number | null;
  enabled?: boolean | null;
}

export interface TestFireResult {
  ok: boolean;
  statusCode?: number;
  error?: string;
}
