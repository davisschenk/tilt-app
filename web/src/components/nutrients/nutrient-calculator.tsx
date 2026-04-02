import { useState, useEffect, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useNutrientCalculate, useCreateNutrientSchedule } from "@/hooks/use-nutrient-schedule";
import { useAlertTargets } from "@/hooks/use-alert-targets";
import type {
  NitrogenRequirement,
  NutrientProtocol,
  NutrientCalculateResponse,
} from "@/types";
import { Beaker, AlertTriangle, Info } from "lucide-react";

interface NutrientCalculatorProps {
  brewId: string;
  brewOg: number | null;
  onCreated: () => void;
  onCancel: () => void;
}

const PROTOCOL_LABELS: Record<NutrientProtocol, string> = {
  fermaid_o: "Fermaid O Only",
  fermaid_ok: "Fermaid O + K",
  fermaid_okdap: "Fermaid O + K + DAP",
};

const NITROGEN_HINTS: Record<NitrogenRequirement, string> = {
  low: "71B, EC-1118, Fleischmann's",
  medium: "K1V-1116, D21, Wyeast 1388",
  high: "D47",
};

export function NutrientCalculator({
  brewId,
  brewOg,
  onCreated,
  onCancel,
}: NutrientCalculatorProps) {
  const [og, setOg] = useState(brewOg?.toString() ?? "1.100");
  const [batchSize, setBatchSize] = useState("5");
  const [nitrogenReq, setNitrogenReq] = useState<NitrogenRequirement>("medium");
  const [protocol, setProtocol] = useState<NutrientProtocol>("fermaid_o");
  const [goFermOffset, setGoFermOffset] = useState(false);
  const [fruitOffset, setFruitOffset] = useState("0");
  const [alertTargetId, setAlertTargetId] = useState<string | null>(null);
  const [preview, setPreview] = useState<NutrientCalculateResponse | null>(null);

  const calculate = useNutrientCalculate();
  const createSchedule = useCreateNutrientSchedule(brewId);
  const { data: alertTargets } = useAlertTargets();

  const doCalculate = useCallback(() => {
    const ogNum = parseFloat(og);
    const batchNum = parseFloat(batchSize);
    const fruitNum = parseFloat(fruitOffset) || 0;
    if (isNaN(ogNum) || isNaN(batchNum) || ogNum < 1.0 || batchNum <= 0) return;

    calculate.mutate(
      {
        og: ogNum,
        batchSizeGallons: batchNum,
        nitrogenRequirement: nitrogenReq,
        nutrientProtocol: protocol,
        goFermOffset,
        fruitOffsetPpm: fruitNum,
      },
      { onSuccess: (data) => setPreview(data) }
    );
  }, [og, batchSize, nitrogenReq, protocol, goFermOffset, fruitOffset, calculate]);

  useEffect(() => {
    const timer = setTimeout(doCalculate, 300);
    return () => clearTimeout(timer);
  }, [doCalculate]);

  const handleSave = () => {
    createSchedule.mutate(
      {
        og: parseFloat(og),
        batchSizeGallons: parseFloat(batchSize),
        nitrogenRequirement: nitrogenReq,
        nutrientProtocol: protocol,
        goFermOffset,
        fruitOffsetPpm: parseFloat(fruitOffset) || 0,
        alertTargetId,
      },
      { onSuccess: onCreated }
    );
  };

  const showK = protocol !== "fermaid_o";
  const showDap = protocol === "fermaid_okdap";

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Beaker className="h-5 w-5" />
          Nutrient Calculator
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Inputs */}
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="og">Original Gravity</Label>
            <Input
              id="og"
              type="number"
              step="0.001"
              min="1.000"
              max="1.200"
              value={og}
              onChange={(e) => setOg(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="batch-size">Batch Size (gallons)</Label>
            <Input
              id="batch-size"
              type="number"
              step="0.5"
              min="0.5"
              value={batchSize}
              onChange={(e) => setBatchSize(e.target.value)}
            />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label>Nitrogen Requirement</Label>
            <Select
              value={nitrogenReq}
              onValueChange={(v) => setNitrogenReq(v as NitrogenRequirement)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(["low", "medium", "high"] as NitrogenRequirement[]).map(
                  (level) => (
                    <SelectItem key={level} value={level}>
                      <span className="capitalize">{level}</span>
                      <span className="text-muted-foreground ml-2 text-xs">
                        ({NITROGEN_HINTS[level]})
                      </span>
                    </SelectItem>
                  )
                )}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label>Nutrient Protocol</Label>
            <Select
              value={protocol}
              onValueChange={(v) => setProtocol(v as NutrientProtocol)}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {(
                  Object.entries(PROTOCOL_LABELS) as [
                    NutrientProtocol,
                    string,
                  ][]
                ).map(([value, label]) => (
                  <SelectItem key={value} value={value}>
                    {label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="flex items-center gap-3">
            <Switch
              checked={goFermOffset}
              onCheckedChange={setGoFermOffset}
              id="goferm"
            />
            <Label htmlFor="goferm" className="cursor-pointer">
              GoFerm YAN offset
            </Label>
          </div>
          <div className="space-y-2">
            <Label htmlFor="fruit-offset" className="flex items-center gap-1">
              Fruit YAN offset (ppm)
              <span className="text-muted-foreground text-xs" title="~25 ppm per lb/gal of fruit">
                <Info className="h-3 w-3 inline" />
              </span>
            </Label>
            <Input
              id="fruit-offset"
              type="number"
              step="5"
              min="0"
              value={fruitOffset}
              onChange={(e) => setFruitOffset(e.target.value)}
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label>Webhook Notifications (optional)</Label>
          <Select
            value={alertTargetId ?? "none"}
            onValueChange={(v) =>
              setAlertTargetId(v === "none" ? null : v)
            }
          >
            <SelectTrigger>
              <SelectValue placeholder="No notifications" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="none">No notifications</SelectItem>
              {alertTargets
                ?.filter((t) => t.enabled)
                .map((t) => (
                  <SelectItem key={t.id} value={t.id}>
                    {t.name}
                  </SelectItem>
                ))}
            </SelectContent>
          </Select>
        </div>

        {/* Preview Results */}
        {preview && (
          <div className="space-y-4 border-t pt-4">
            {preview.maxDosageCapped && (
              <div className="bg-yellow-50 dark:bg-yellow-950 border border-yellow-200 dark:border-yellow-800 rounded-md p-3 flex items-center gap-2 text-sm">
                <AlertTriangle className="h-4 w-4 text-yellow-600" />
                <span>
                  YAN target exceeds max nutrient dosages. Amounts are
                  capped at safe limits.
                </span>
              </div>
            )}

            <div className="grid grid-cols-3 gap-3 text-sm">
              <div>
                <span className="text-muted-foreground">Target YAN</span>
                <p className="font-medium">{preview.totalYanPpm.toFixed(0)} ppm</p>
              </div>
              <div>
                <span className="text-muted-foreground">Effective YAN</span>
                <p className="font-medium">{preview.effectiveYanPpm.toFixed(0)} ppm</p>
              </div>
              <div>
                <span className="text-muted-foreground">1/3 Sugar Break</span>
                <p className="font-medium">{preview.oneThirdBreakSg.toFixed(3)}</p>
              </div>
            </div>

            <div className="grid grid-cols-3 gap-3 text-sm">
              <div>
                <span className="text-muted-foreground">Yeast</span>
                <p className="font-medium">{preview.yeastGrams}g</p>
              </div>
              <div>
                <span className="text-muted-foreground">GoFerm</span>
                <p className="font-medium">{preview.goFermGrams}g</p>
              </div>
              <div>
                <span className="text-muted-foreground">Rehydration Water</span>
                <p className="font-medium">{preview.rehydrationWaterMl}mL</p>
              </div>
            </div>

            {/* Addition Schedule Table */}
            <div className="rounded-md border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b bg-muted/50">
                    <th className="px-3 py-2 text-left font-medium">#</th>
                    <th className="px-3 py-2 text-left font-medium">Fermaid O</th>
                    {showK && (
                      <th className="px-3 py-2 text-left font-medium">Fermaid K</th>
                    )}
                    {showDap && (
                      <th className="px-3 py-2 text-left font-medium">DAP</th>
                    )}
                    <th className="px-3 py-2 text-left font-medium">Trigger</th>
                  </tr>
                </thead>
                <tbody>
                  {preview.additions.map((a) => (
                    <tr key={a.additionNumber} className="border-b last:border-0">
                      <td className="px-3 py-2">{a.additionNumber}</td>
                      <td className="px-3 py-2">{a.fermaidOGrams.toFixed(1)}g</td>
                      {showK && (
                        <td className="px-3 py-2">{a.fermaidKGrams.toFixed(1)}g</td>
                      )}
                      {showDap && (
                        <td className="px-3 py-2">{a.dapGrams.toFixed(1)}g</td>
                      )}
                      <td className="px-3 py-2 text-muted-foreground">
                        {a.triggerType === "time"
                          ? `${a.targetHours}h after pitch`
                          : `SG ≤ ${a.targetGravity?.toFixed(3)} or day 7`}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel}>
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={createSchedule.isPending || !preview}>
            {createSchedule.isPending ? "Saving..." : "Save Schedule"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
