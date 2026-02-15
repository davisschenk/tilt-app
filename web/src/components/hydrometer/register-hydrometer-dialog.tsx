import { useState, useMemo } from "react";
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
import ColorDot from "@/components/ui/color-dot";
import { useHydrometers, useCreateHydrometer } from "@/hooks/use-hydrometers";
import { ALL_TILT_COLORS } from "@/lib/tilt-colors";
import * as toast from "@/lib/toast";

interface RegisterHydrometerDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function RegisterHydrometerDialog({
  open,
  onOpenChange,
}: RegisterHydrometerDialogProps) {
  const { data: hydrometers } = useHydrometers();
  const createHydrometer = useCreateHydrometer();

  const [color, setColor] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState("");

  const availableColors = useMemo(() => {
    const registered = new Set(hydrometers?.map((h) => h.color) ?? []);
    return ALL_TILT_COLORS.filter((c) => !registered.has(c));
  }, [hydrometers]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!color) {
      setError("Color is required");
      return;
    }
    setError("");

    createHydrometer.mutate(
      {
        color: color as any,
        name: name.trim() || null,
      },
      {
        onSuccess: () => {
          toast.success("Hydrometer registered");
          setColor("");
          setName("");
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to register hydrometer");
        },
      },
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Register Hydrometer</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="reg-color">Color *</Label>
            <Select value={color} onValueChange={setColor}>
              <SelectTrigger id="reg-color">
                <SelectValue placeholder="Select a Tilt color" />
              </SelectTrigger>
              <SelectContent>
                {availableColors.map((c) => (
                  <SelectItem key={c} value={c}>
                    <div className="flex items-center gap-2">
                      <ColorDot color={c} />
                      <span>{c}</span>
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            {error && <p className="text-sm text-destructive">{error}</p>}
          </div>

          <div className="space-y-2">
            <Label htmlFor="reg-name">Name / Alias</Label>
            <Input
              id="reg-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Optional alias"
            />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={createHydrometer.isPending}>
              {createHydrometer.isPending ? "Registering..." : "Register"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
