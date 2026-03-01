import { useState } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
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
import { Switch } from "@/components/ui/switch";
import { useCreateAlertRule } from "@/hooks/use-alert-rules";
import { useAlertTargets } from "@/hooks/use-alert-targets";
import { useBrews } from "@/hooks/use-brews";
import { useHydrometers } from "@/hooks/use-hydrometers";
import * as toast from "@/lib/toast";
import type { AlertMetric, AlertOperator } from "@/types";

interface CreateAlertRuleDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  defaultBrewId?: string;
}

export default function CreateAlertRuleDialog({
  open,
  onOpenChange,
  defaultBrewId,
}: CreateAlertRuleDialogProps) {
  const createRule = useCreateAlertRule();
  const { data: targets } = useAlertTargets();
  const { data: brews } = useBrews("Active");
  const { data: hydrometers } = useHydrometers();

  const [name, setName] = useState("");
  const [metric, setMetric] = useState<AlertMetric>("gravity");
  const [operator, setOperator] = useState<AlertOperator>("lte");
  const [threshold, setThreshold] = useState("");
  const [alertTargetId, setAlertTargetId] = useState("");
  const [brewId, setBrewId] = useState(defaultBrewId ?? "");
  const [hydrometerId, setHydrometerId] = useState("");
  const [cooldownMinutes, setCooldownMinutes] = useState("60");
  const [enabled, setEnabled] = useState(true);
  const [error, setError] = useState("");

  function resetForm() {
    setName("");
    setMetric("gravity");
    setOperator("lte");
    setThreshold("");
    setAlertTargetId("");
    setBrewId(defaultBrewId ?? "");
    setHydrometerId("");
    setCooldownMinutes("60");
    setEnabled(true);
    setError("");
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) { setError("Name is required"); return; }
    if (!threshold) { setError("Threshold is required"); return; }
    if (!alertTargetId) { setError("Webhook target is required"); return; }
    setError("");

    createRule.mutate(
      {
        name: name.trim(),
        metric,
        operator,
        threshold: parseFloat(threshold),
        alertTargetId,
        brewId: brewId || null,
        hydrometerId: hydrometerId || null,
        cooldownMinutes: parseInt(cooldownMinutes) || 60,
        enabled,
      },
      {
        onSuccess: () => {
          toast.success("Alert rule created");
          resetForm();
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to create alert rule");
        },
      },
    );
  }

  const thresholdStep = metric === "gravity" ? "0.001" : "0.5";

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) resetForm(); onOpenChange(o); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Add Alert Rule</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="rule-name">Name *</Label>
            <Input id="rule-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. FG Reached" />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="rule-metric">Metric *</Label>
              <Select value={metric} onValueChange={(v) => setMetric(v as AlertMetric)}>
                <SelectTrigger id="rule-metric"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="gravity">Gravity</SelectItem>
                  <SelectItem value="temperature_f">Temperature (°F)</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="rule-operator">Operator *</Label>
              <Select value={operator} onValueChange={(v) => setOperator(v as AlertOperator)}>
                <SelectTrigger id="rule-operator"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="lte">≤ (less or equal)</SelectItem>
                  <SelectItem value="gte">≥ (greater or equal)</SelectItem>
                  <SelectItem value="lt">&lt; (less than)</SelectItem>
                  <SelectItem value="gt">&gt; (greater than)</SelectItem>
                  <SelectItem value="eq">= (equal)</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="rule-threshold">Threshold *</Label>
            <Input
              id="rule-threshold"
              type="number"
              step={thresholdStep}
              value={threshold}
              onChange={(e) => setThreshold(e.target.value)}
              placeholder={metric === "gravity" ? "1.010" : "72.0"}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="rule-target">Webhook Target *</Label>
            <Select value={alertTargetId} onValueChange={setAlertTargetId}>
              <SelectTrigger id="rule-target"><SelectValue placeholder="Select a target" /></SelectTrigger>
              <SelectContent>
                {targets?.map((t) => (
                  <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="rule-brew">Brew (optional)</Label>
              <Select value={brewId} onValueChange={setBrewId}>
                <SelectTrigger id="rule-brew"><SelectValue placeholder="Any brew" /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="">Any brew</SelectItem>
                  {brews?.map((b) => (
                    <SelectItem key={b.id} value={b.id}>{b.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="rule-hydro">Hydrometer (optional)</Label>
              <Select value={hydrometerId} onValueChange={setHydrometerId}>
                <SelectTrigger id="rule-hydro"><SelectValue placeholder="Any hydrometer" /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="">Any hydrometer</SelectItem>
                  {hydrometers?.map((h) => (
                    <SelectItem key={h.id} value={h.id}>{h.name ?? h.color}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="rule-cooldown">Cooldown (minutes)</Label>
            <Input
              id="rule-cooldown"
              type="number"
              min="1"
              value={cooldownMinutes}
              onChange={(e) => setCooldownMinutes(e.target.value)}
            />
          </div>

          <div className="flex items-center justify-between">
            <Label htmlFor="rule-enabled">Enabled</Label>
            <Switch id="rule-enabled" checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => { resetForm(); onOpenChange(false); }}>Cancel</Button>
            <Button type="submit" disabled={createRule.isPending}>
              {createRule.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
