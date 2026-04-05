import { useState } from "react";
import { format } from "date-fns";
import { FlaskConical, ChevronDown, ChevronUp, Info, Settings2 } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import NutrientScheduleTable from "./nutrient-schedule-table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useUpdateBrewNutrientSetup } from "@/hooks/use-nutrient-schedule";
import { useAlertTargets } from "@/hooks/use-alert-targets";
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
  medium: "Medium (e.g. Lalvin D47, Mangrove Jack M05)",
  high: "High",
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
  const [yeastStrain, setYeastStrain] = useState(brew.yeastStrain ?? "");
  const [alertTargetId, setAlertTargetId] = useState<string>(
    brew.nutrientAlertTargetId ?? "none",
  );

  const { data: alertTargets } = useAlertTargets();
  const update = useUpdateBrewNutrientSetup(brew.id);

  const isConfigured =
    brew.batchSizeGallons != null &&
    brew.pitchTime != null &&
    brew.og != null &&
    brew.targetFg != null;

  const pitchTimeDisplay = brew.pitchTime
    ? format(new Date(brew.pitchTime), "MMM d, yyyy 'at' HH:mm")
    : null;

  function handleSave() {
    const batchNum = parseFloat(batchSize);
    if (isNaN(batchNum) || batchNum <= 0) {
      toast.error("Batch size must be a positive number");
      return;
    }
    update.mutate(
      {
        batchSizeGallons: batchNum,
        yeastNitrogenRequirement: nitrogenReq,
        nutrientProtocol: protocol,
        yeastStrain: yeastStrain.trim() || null,
        nutrientAlertTargetId: alertTargetId === "none" ? null : alertTargetId,
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
          <div className="flex items-center gap-1.5 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
            <Settings2 className="h-3.5 w-3.5" />
            Settings
          </div>
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
              <Label>Pitch Time</Label>
              <div className="flex items-start gap-2 rounded-md border bg-muted/40 px-3 py-2 text-sm">
                <Info className="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground" />
                <span className="text-muted-foreground">
                  {pitchTimeDisplay ? (
                    <>
                      <span className="font-medium text-foreground">{pitchTimeDisplay}</span>
                      {" — set from Yeast Pitch event"}
                    </>
                  ) : (
                    "Set automatically when you log a Yeast Pitch event below"
                  )}
                </span>
              </div>
            </div>

            <div className="space-y-1.5 sm:col-span-2">
              <Label htmlFor="alert-target">Webhook Alert Target</Label>
              <Select
                value={alertTargetId}
                onValueChange={setAlertTargetId}
              >
                <SelectTrigger id="alert-target">
                  <SelectValue placeholder="None (no webhook notifications)" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">None (no notifications)</SelectItem>
                  {alertTargets?.map((t) => (
                    <SelectItem key={t.id} value={t.id}>
                      {t.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-xs text-muted-foreground">
                Nutrient addition and temperature warnings will be sent to this target only.
              </p>
            </div>

            <div className="space-y-1.5 sm:col-span-2">
              <Label htmlFor="yeast-strain">Yeast Strain (optional)</Label>
              <Input
                id="yeast-strain"
                type="text"
                placeholder="e.g. 71B, D47, EC-1118"
                value={yeastStrain}
                onChange={(e) => setYeastStrain(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">
                If set and Nitrogen Requirement is left as default, it will be auto-detected from the strain.
              </p>
            </div>
          </div>

          <div className="flex justify-end">
            <Button onClick={handleSave} disabled={update.isPending}>
              {update.isPending ? "Saving…" : "Save Settings"}
            </Button>
          </div>

          <NutrientScheduleTable brew={brew} />
        </CardContent>
      )}
    </Card>
  );
}
