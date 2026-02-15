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
import { useUpdateHydrometer } from "@/hooks/use-hydrometers";
import * as toast from "@/lib/toast";
import type { HydrometerResponse } from "@/types";

interface EditHydrometerDialogProps {
  hydrometer: HydrometerResponse;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function EditHydrometerDialog({
  hydrometer,
  open,
  onOpenChange,
}: EditHydrometerDialogProps) {
  const updateHydrometer = useUpdateHydrometer(hydrometer.id);

  const [name, setName] = useState(hydrometer.name ?? "");
  const [tempOffset, setTempOffset] = useState(
    hydrometer.tempOffsetF?.toString() ?? "0",
  );
  const [gravityOffset, setGravityOffset] = useState(
    hydrometer.gravityOffset?.toString() ?? "0",
  );

  useEffect(() => {
    if (open) {
      setName(hydrometer.name ?? "");
      setTempOffset(hydrometer.tempOffsetF?.toString() ?? "0");
      setGravityOffset(hydrometer.gravityOffset?.toString() ?? "0");
    }
  }, [open, hydrometer]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    updateHydrometer.mutate(
      {
        name: name.trim() || null,
        tempOffsetF: tempOffset ? parseFloat(tempOffset) : 0,
        gravityOffset: gravityOffset ? parseFloat(gravityOffset) : 0,
      },
      {
        onSuccess: () => {
          toast.success("Hydrometer updated");
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to update hydrometer");
        },
      },
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Edit {hydrometer.color} Hydrometer</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="edit-h-name">Name / Alias</Label>
            <Input
              id="edit-h-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Optional alias"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-h-temp-offset">Temperature Offset (°F)</Label>
            <Input
              id="edit-h-temp-offset"
              type="number"
              step="0.1"
              value={tempOffset}
              onChange={(e) => setTempOffset(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Adjusts raw temperature readings from this hydrometer. Positive values increase the reported temperature.
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-h-gravity-offset">Gravity Offset</Label>
            <Input
              id="edit-h-gravity-offset"
              type="number"
              step="0.001"
              value={gravityOffset}
              onChange={(e) => setGravityOffset(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Adjusts raw gravity readings from this hydrometer. Use this to calibrate against a known reference.
            </p>
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={updateHydrometer.isPending}>
              {updateHydrometer.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
