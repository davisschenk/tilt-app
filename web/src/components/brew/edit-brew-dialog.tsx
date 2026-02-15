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
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useUpdateBrew } from "@/hooks/use-brews";
import * as toast from "@/lib/toast";
import type { BrewResponse, BrewStatus } from "@/types";

interface EditBrewDialogProps {
  brew: BrewResponse;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function EditBrewDialog({ brew, open, onOpenChange }: EditBrewDialogProps) {
  const updateBrew = useUpdateBrew(brew.id);

  const [name, setName] = useState(brew.name);
  const [style, setStyle] = useState(brew.style ?? "");
  const [og, setOg] = useState(brew.og?.toString() ?? "");
  const [fg, setFg] = useState(brew.fg?.toString() ?? "");
  const [targetFg, setTargetFg] = useState(brew.targetFg?.toString() ?? "");
  const [abv, setAbv] = useState(brew.abv?.toString() ?? "");
  const [notes, setNotes] = useState(brew.notes ?? "");
  const [status, setStatus] = useState<BrewStatus>(brew.status);

  useEffect(() => {
    if (open) {
      setName(brew.name);
      setStyle(brew.style ?? "");
      setOg(brew.og?.toString() ?? "");
      setFg(brew.fg?.toString() ?? "");
      setTargetFg(brew.targetFg?.toString() ?? "");
      setAbv(brew.abv?.toString() ?? "");
      setNotes(brew.notes ?? "");
      setStatus(brew.status);
    }
  }, [open, brew]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    updateBrew.mutate(
      {
        name: name.trim() || undefined,
        style: style.trim() || null,
        og: og ? parseFloat(og) : null,
        fg: fg ? parseFloat(fg) : null,
        targetFg: targetFg ? parseFloat(targetFg) : null,
        abv: abv ? parseFloat(abv) : null,
        notes: notes.trim() || null,
        status,
      },
      {
        onSuccess: () => {
          toast.success("Brew updated");
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to update brew");
        },
      },
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Edit Brew</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="edit-name">Name</Label>
            <Input id="edit-name" value={name} onChange={(e) => setName(e.target.value)} />
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-style">Style</Label>
            <Input id="edit-style" value={style} onChange={(e) => setStyle(e.target.value)} />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="edit-og">OG</Label>
              <Input id="edit-og" type="number" step="0.001" value={og} onChange={(e) => setOg(e.target.value)} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-fg">FG</Label>
              <Input id="edit-fg" type="number" step="0.001" value={fg} onChange={(e) => setFg(e.target.value)} />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="edit-target-fg">Target FG</Label>
              <Input id="edit-target-fg" type="number" step="0.001" value={targetFg} onChange={(e) => setTargetFg(e.target.value)} />
            </div>
            <div className="space-y-2">
              <Label htmlFor="edit-abv">ABV (%)</Label>
              <Input id="edit-abv" type="number" step="0.1" value={abv} onChange={(e) => setAbv(e.target.value)} />
            </div>
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-status">Status</Label>
            <Select value={status} onValueChange={(v) => setStatus(v as BrewStatus)}>
              <SelectTrigger id="edit-status">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="Active">Active</SelectItem>
                <SelectItem value="Completed">Completed</SelectItem>
                <SelectItem value="Archived">Archived</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-notes">Notes</Label>
            <Textarea id="edit-notes" value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={updateBrew.isPending}>
              {updateBrew.isPending ? "Saving..." : "Save Changes"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
