import { useUnits } from "@/lib/units";
import { MeasurementInput } from "@/components/ui/measurement-input";
import { useState } from "react";
import { useSearchParams } from "react-router-dom";
import { useBrew } from "@/hooks/use-brews";
import type { BrewResponse } from "@/types";
import { useNavigate } from "react-router-dom";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent } from "@/components/ui/card";
import { useCreateBrew, useBrews } from "@/hooks/use-brews";
import { useHydrometers } from "@/hooks/use-hydrometers";
import ColorDot from "@/components/ui/color-dot";
import * as toast from "@/lib/toast";

export default function BrewNew() {
 const [params]=useSearchParams(); const sourceId=params.get("duplicate") ?? "";
 const source=useBrew(sourceId);
 if(sourceId && source.isPending) return <p>Loading original batch…</p>;
 if(sourceId && source.isError) return <p>Could not load the original batch.</p>;
 return <BrewForm key={sourceId} source={source.data} />;
}
function BrewForm({source}:{source?:BrewResponse}) {
  const u = useUnits();
  const navigate = useNavigate();
  const { data: hydrometers } = useHydrometers();
  const { data: activeBrews } = useBrews("Active");
  const createBrew = useCreateBrew();

  const usedHydrometerIds = new Set(activeBrews?.map((b) => b.hydrometerId) ?? []);

  const [name, setName] = useState(source ? `${source.name} (copy)` : "");
  const [style, setStyle] = useState(source?.style ?? "");
  const [hydrometerId, setHydrometerId] = useState("");
  const [og, setOg] = useState(source?.og?.toString() ?? "");
  const [targetFg, setTargetFg] = useState(source?.targetFg?.toString() ?? "");
  const [notes, setNotes] = useState(source?.notes ?? "");
  const [errors, setErrors] = useState<Record<string, string>>({});

  function handleHydrometerChange(id: string) {
    setHydrometerId(id);
    const selected = hydrometers?.find((h) => h.id === id);
    if (selected?.latestReading && !og) {
      setOg(selected.latestReading.gravity.toFixed(3));
    }
  }

  function validate(): boolean {
    const errs: Record<string, string> = {};
    if (!name.trim()) errs.name = "Name is required";
    if (!hydrometerId) errs.hydrometerId = "Hydrometer is required";
    if (usedHydrometerIds.has(hydrometerId) || hydrometers?.find(h=>h.id===hydrometerId)?.isDisabled) errs.hydrometerId="Choose an available, enabled hydrometer";
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;

    createBrew.mutate(
      {
        batchSizeGallons: source?.batchSizeGallons,
        yeastStrain: source?.yeastStrain,
        yeastNitrogenRequirement: source?.yeastNitrogenRequirement,
        nutrientProtocol: source?.nutrientProtocol,
        name: name.trim(),
        hydrometerId,
        style: style.trim() || null,
        og: og ? parseFloat(og) : null,
        targetFg: targetFg ? parseFloat(targetFg) : null,
        notes: notes.trim() || null,
      },
      {
        onSuccess: (brew) => {
          toast.success("Brew created successfully");
          navigate(`/brews/${brew.id}`);
        },
        onError: () => {
          toast.error("Failed to create brew");
        },
      },
    );
  }

  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title={source ? "Duplicate Brew" : "New Brew"}
        description="Create a new brew."
      />

      {source && <p className="mb-4">Review the copied recipe and select a hydrometer. Yeast: {source.yeastStrain ?? "—"}; batch size: {source.batchSizeGallons ?? "—"} gal; nutrient protocol: {source.nutrientProtocol ?? "—"}. Dates, readings and log entries start fresh.</p>}
      <Card className="max-w-2xl">
        <CardContent className="pt-6">
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="name">Name *</Label>
              <Input
                id="name"
                value={name}
                onChange={(e) => setName(e.target.value)}
                placeholder="My IPA"
              />
              {errors.name && (
                <p className="text-sm text-destructive">{errors.name}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="style">Style</Label>
              <Input
                id="style"
                value={style}
                onChange={(e) => setStyle(e.target.value)}
                placeholder="American IPA"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="hydrometer">Hydrometer *</Label>
              <Select value={hydrometerId} onValueChange={handleHydrometerChange}>
                <SelectTrigger id="hydrometer">
                  <SelectValue placeholder="Select a hydrometer" />
                </SelectTrigger>
                <SelectContent>
                  {hydrometers?.map((h) => {
                    const inUse = usedHydrometerIds.has(h.id) || h.isDisabled;
                    if (inUse) {
                      return (
                        <div
                          key={h.id}
                          className="flex items-center gap-2 rounded-sm py-1.5 pr-8 pl-2 text-sm opacity-50 cursor-not-allowed select-none"
                        >
                          <ColorDot color={h.color} />
                          {h.name || h.color}
                          <span className="text-xs text-muted-foreground">(in use)</span>
                        </div>
                      );
                    }
                    return (
                      <SelectItem key={h.id} value={h.id}>
                        <span className="flex items-center gap-2">
                          <ColorDot color={h.color} />
                          {h.name || h.color}
                        </span>
                      </SelectItem>
                    );
                  })}
                </SelectContent>
              </Select>
              {hydrometers && !hydrometers.some(h=>!h.isDisabled && !usedHydrometerIds.has(h.id)) && <p className="text-sm text-muted-foreground">No hydrometer is available. Finish an active batch or register an enabled hydrometer first.</p>}
              {errors.hydrometerId && (
                <p className="text-sm text-destructive">{errors.hydrometerId}</p>
              )}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="og">Original Gravity ({u.gravityLabel})</Label>
                <MeasurementInput measurement="gravity"
                  id="og"
                  type="number"
                  step="0.001"
                  min="0.990"
                  max="1.200"
                  value={og}
                  onChange={(e) => setOg(e.target.value)}
                  placeholder="1.050"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="targetFg">Target FG ({u.gravityLabel})</Label>
                <MeasurementInput measurement="gravity"
                  id="targetFg"
                  type="number"
                  step="0.001"
                  min="0.990"
                  max="1.200"
                  value={targetFg}
                  onChange={(e) => setTargetFg(e.target.value)}
                  placeholder="1.010"
                />
              </div>
            </div>

            <div className="space-y-2">
              <Label htmlFor="notes">Notes</Label>
              <Textarea
                id="notes"
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                placeholder="Recipe notes, ingredients, etc."
                rows={4}
              />
            </div>

            <div className="flex gap-2 pt-2">
              <Button type="submit" disabled={createBrew.isPending}>
                {createBrew.isPending ? "Creating..." : "Create Brew"}
              </Button>
              <Button
                type="button"
                variant="outline"
                onClick={() => navigate("/brews")}
              >
                Cancel
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
