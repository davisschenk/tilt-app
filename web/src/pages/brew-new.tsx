import { useState } from "react";
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
  const navigate = useNavigate();
  const { data: hydrometers } = useHydrometers();
  const { data: activeBrews } = useBrews("Active");
  const createBrew = useCreateBrew();

  const usedHydrometerIds = new Set(activeBrews?.map((b) => b.hydrometerId) ?? []);

  const [name, setName] = useState("");
  const [style, setStyle] = useState("");
  const [hydrometerId, setHydrometerId] = useState("");
  const [og, setOg] = useState("");
  const [targetFg, setTargetFg] = useState("");
  const [notes, setNotes] = useState("");
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
    setErrors(errs);
    return Object.keys(errs).length === 0;
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!validate()) return;

    createBrew.mutate(
      {
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
        title="New Brew"
        description="Create a new brew."
      />

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
                    const inUse = usedHydrometerIds.has(h.id);
                    return (
                      <SelectItem key={h.id} value={h.id} disabled={inUse}>
                        <span className="flex items-center gap-2">
                          <ColorDot color={h.color} />
                          {h.name || h.color}
                          {inUse && <span className="text-xs text-muted-foreground">(in use)</span>}
                        </span>
                      </SelectItem>
                    );
                  })}
                </SelectContent>
              </Select>
              {errors.hydrometerId && (
                <p className="text-sm text-destructive">{errors.hydrometerId}</p>
              )}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="og">Original Gravity</Label>
                <Input
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
                <Label htmlFor="targetFg">Target FG</Label>
                <Input
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
