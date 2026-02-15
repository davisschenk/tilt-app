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
