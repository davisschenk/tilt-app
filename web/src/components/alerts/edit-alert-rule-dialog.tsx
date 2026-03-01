import { useState, useEffect } from "react";
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
import { useUpdateAlertRule } from "@/hooks/use-alert-rules";
import { useAlertTargets } from "@/hooks/use-alert-targets";
import { useBrews } from "@/hooks/use-brews";
import { useHydrometers } from "@/hooks/use-hydrometers";
import * as toast from "@/lib/toast";
import type { AlertRuleResponse, AlertMetric, AlertOperator } from "@/types";

interface EditAlertRuleDialogProps {
  rule: AlertRuleResponse;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function EditAlertRuleDialog({
  rule,
  open,
  onOpenChange,
}: EditAlertRuleDialogProps) {
  const updateRule = useUpdateAlertRule(rule.id);
  const { data: targets } = useAlertTargets();
  const { data: brews } = useBrews("Active");
  const { data: hydrometers } = useHydrometers();

  const [name, setName] = useState(rule.name);
  const [metric, setMetric] = useState<AlertMetric>(rule.metric);
  const [operator, setOperator] = useState<AlertOperator>(rule.operator);
  const [threshold, setThreshold] = useState(String(rule.threshold));
  const [alertTargetId, setAlertTargetId] = useState(rule.alertTargetId);
  const [brewId, setBrewId] = useState(rule.brewId ?? "");
  const [hydrometerId, setHydrometerId] = useState(rule.hydrometerId ?? "");
  const [cooldownMinutes, setCooldownMinutes] = useState(String(rule.cooldownMinutes));
  const [enabled, setEnabled] = useState(rule.enabled);
  const [error, setError] = useState("");

  useEffect(() => {
    setName(rule.name);
    setMetric(rule.metric);
    setOperator(rule.operator);
    setThreshold(String(rule.threshold));
    setAlertTargetId(rule.alertTargetId);
    setBrewId(rule.brewId ?? "");
    setHydrometerId(rule.hydrometerId ?? "");
    setCooldownMinutes(String(rule.cooldownMinutes));
    setEnabled(rule.enabled);
    setError("");
  }, [rule]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) { setError("Name is required"); return; }
    if (!threshold) { setError("Threshold is required"); return; }
    if (!alertTargetId) { setError("Webhook target is required"); return; }
    setError("");

    updateRule.mutate(
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
          toast.success("Alert rule updated");
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to update alert rule");
        },
      },
    );
  }

  const thresholdStep = metric === "gravity" ? "0.001" : "0.5";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Edit Alert Rule</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="edit-rule-name">Name *</Label>
            <Input id="edit-rule-name" value={name} onChange={(e) => setName(e.target.value)} />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="edit-rule-metric">Metric *</Label>
              <Select value={metric} onValueChange={(v) => setMetric(v as AlertMetric)}>
                <SelectTrigger id="edit-rule-metric"><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="gravity">Gravity</SelectItem>
                  <SelectItem value="temperature_f">Temperature (°F)</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-rule-operator">Operator *</Label>
              <Select value={operator} onValueChange={(v) => setOperator(v as AlertOperator)}>
                <SelectTrigger id="edit-rule-operator"><SelectValue /></SelectTrigger>
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
            <Label htmlFor="edit-rule-threshold">Threshold *</Label>
            <Input
              id="edit-rule-threshold"
              type="number"
              step={thresholdStep}
              value={threshold}
              onChange={(e) => setThreshold(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-rule-target">Webhook Target *</Label>
            <Select value={alertTargetId} onValueChange={setAlertTargetId}>
              <SelectTrigger id="edit-rule-target"><SelectValue /></SelectTrigger>
              <SelectContent>
                {targets?.map((t) => (
                  <SelectItem key={t.id} value={t.id}>{t.name}</SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-2">
              <Label htmlFor="edit-rule-brew">Brew (optional)</Label>
              <Select value={brewId || "__any__"} onValueChange={(v) => setBrewId(v === "__any__" ? "" : v)}>
                <SelectTrigger id="edit-rule-brew"><SelectValue placeholder="Any brew" /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="__any__">Any brew</SelectItem>
                  {brews?.map((b) => (
                    <SelectItem key={b.id} value={b.id}>{b.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-rule-hydro">Hydrometer (optional)</Label>
              <Select value={hydrometerId || "__any__"} onValueChange={(v) => setHydrometerId(v === "__any__" ? "" : v)}>
                <SelectTrigger id="edit-rule-hydro"><SelectValue placeholder="Any hydrometer" /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="__any__">Any hydrometer</SelectItem>
                  {hydrometers?.map((h) => (
                    <SelectItem key={h.id} value={h.id}>{h.name ?? h.color}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-rule-cooldown">Cooldown (minutes)</Label>
            <Input
              id="edit-rule-cooldown"
              type="number"
              min="1"
              value={cooldownMinutes}
              onChange={(e) => setCooldownMinutes(e.target.value)}
            />
          </div>

          <div className="flex items-center justify-between">
            <Label htmlFor="edit-rule-enabled">Enabled</Label>
            <Switch id="edit-rule-enabled" checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
            <Button type="submit" disabled={updateRule.isPending}>
              {updateRule.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
