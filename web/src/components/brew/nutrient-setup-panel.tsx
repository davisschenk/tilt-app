import { useState } from "react";
import { FlaskConical, ChevronDown, ChevronUp } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useUpdateBrewNutrientSetup } from "@/hooks/use-nutrient-schedule";
import * as toast from "@/lib/toast";
import type { BrewResponse } from "@/types";

interface Props {
  brew: BrewResponse;
}

const PROTOCOL_LABELS: Record<string, string> = {
  tosna_2: "TOSNA 2.0 (Fermaid-O only)",
  tosna_3: "TOSNA 3.0 (Fermaid-K + Fermaid-O)",
  advanced_sna: "Advanced SNA (GoFerm + Fermaid-K + Fermaid-O)",
};

const NITROGEN_LABELS: Record<string, string> = {
  low: "Low (e.g. EC-1118, Lalvin 71B)",
  medium: "Medium (e.g. Lalvin D47, Lalvin K1-V1116)",
  high: "High (e.g. Mangrove Jack M05)",
};

export default function NutrientSetupPanel({ brew }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [batchSize, setBatchSize] = useState(
    brew.batchSizeGallons != null ? String(brew.batchSizeGallons) : "",
  );
  const [nitrogenReq, setNitrogenReq] = useState(
    brew.yeastNitrogenRequirement ?? "medium",
  );
  const [protocol, setProtocol] = useState(brew.nutrientProtocol ?? "tosna_2");
  const [pitchTime, setPitchTime] = useState(
    brew.pitchTime ? brew.pitchTime.slice(0, 16) : "",
  );

  const update = useUpdateBrewNutrientSetup(brew.id);

  const isConfigured =
    brew.batchSizeGallons != null &&
    brew.pitchTime != null &&
    brew.og != null &&
    brew.targetFg != null;

  function handleSave() {
    const batchNum = parseFloat(batchSize);
    if (isNaN(batchNum) || batchNum <= 0) {
      toast.error("Batch size must be a positive number");
      return;
    }
    if (!pitchTime) {
      toast.error("Pitch time is required");
      return;
    }
    update.mutate(
      {
        batchSizeGallons: batchNum,
        yeastNitrogenRequirement: nitrogenReq,
        nutrientProtocol: protocol,
        pitchTime: new Date(pitchTime).toISOString(),
      },
      {
        onSuccess: () => toast.success("Nutrient schedule settings saved"),
        onError: () => toast.error("Failed to save nutrient settings"),
      },
    );
  }

  const missingFields: string[] = [];
  if (brew.og == null) missingFields.push("OG");
  if (brew.targetFg == null) missingFields.push("Target FG");

  return (
    <Card className="mt-6">
      <CardHeader className="pb-3">
        <button
          type="button"
          className="flex items-center gap-2 w-full text-left"
          onClick={() => setExpanded(!expanded)}
        >
          <FlaskConical className="h-5 w-5 text-muted-foreground" />
          <CardTitle className="text-base flex-1">
            Nutrient Schedule
            {isConfigured && (
              <span className="ml-2 text-xs font-normal text-green-600 dark:text-green-400">
                Configured ({PROTOCOL_LABELS[brew.nutrientProtocol ?? "tosna_2"] ?? brew.nutrientProtocol})
              </span>
            )}
          </CardTitle>
          {expanded ? (
            <ChevronUp className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          )}
        </button>
      </CardHeader>

      {expanded && (
        <CardContent className="space-y-4">
          {missingFields.length > 0 && (
            <div className="rounded-md border border-yellow-300 bg-yellow-50 p-3 text-sm text-yellow-800 dark:border-yellow-800 dark:bg-yellow-950 dark:text-yellow-200">
              Set {missingFields.join(" and ")} on the brew to enable schedule calculation.
            </div>
          )}

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="protocol">Protocol</Label>
              <Select value={protocol} onValueChange={setProtocol}>
                <SelectTrigger id="protocol">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(PROTOCOL_LABELS).map(([val, label]) => (
                    <SelectItem key={val} value={val}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="nitrogen-req">Yeast Nitrogen Requirement</Label>
              <Select value={nitrogenReq} onValueChange={setNitrogenReq}>
                <SelectTrigger id="nitrogen-req">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {Object.entries(NITROGEN_LABELS).map(([val, label]) => (
                    <SelectItem key={val} value={val}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="batch-size">Batch Size (gallons)</Label>
              <Input
                id="batch-size"
                type="number"
                min="0.1"
                step="0.5"
                placeholder="e.g. 1.0"
                value={batchSize}
                onChange={(e) => setBatchSize(e.target.value)}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="pitch-time">Pitch Time</Label>
              <Input
                id="pitch-time"
                type="datetime-local"
                value={pitchTime}
                onChange={(e) => setPitchTime(e.target.value)}
              />
            </div>
          </div>

          <div className="flex justify-end">
            <Button onClick={handleSave} disabled={update.isPending}>
              {update.isPending ? "Saving…" : "Save Settings"}
            </Button>
          </div>
        </CardContent>
      )}
    </Card>
  );
}
