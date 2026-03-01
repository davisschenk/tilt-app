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
import { useUpdateAlertTarget } from "@/hooks/use-alert-targets";
import * as toast from "@/lib/toast";
import type { AlertTargetResponse, WebhookFormat } from "@/types";

interface EditAlertTargetDialogProps {
  target: AlertTargetResponse;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function EditAlertTargetDialog({
  target,
  open,
  onOpenChange,
}: EditAlertTargetDialogProps) {
  const updateTarget = useUpdateAlertTarget(target.id);

  const [name, setName] = useState(target.name);
  const [url, setUrl] = useState(target.url);
  const [format, setFormat] = useState<WebhookFormat>(target.format);
  const [secretHeader, setSecretHeader] = useState(target.secretHeader ?? "");
  const [enabled, setEnabled] = useState(target.enabled);
  const [error, setError] = useState("");

  useEffect(() => {
    setName(target.name);
    setUrl(target.url);
    setFormat(target.format);
    setSecretHeader(target.secretHeader ?? "");
    setEnabled(target.enabled);
    setError("");
  }, [target]);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!name.trim()) {
      setError("Name is required");
      return;
    }
    if (!url.trim()) {
      setError("URL is required");
      return;
    }
    setError("");

    updateTarget.mutate(
      {
        name: name.trim(),
        url: url.trim(),
        format,
        secretHeader: secretHeader.trim() || null,
        enabled,
      },
      {
        onSuccess: () => {
          toast.success("Alert target updated");
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to update alert target");
        },
      },
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Edit Alert Target</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="edit-target-name">Name *</Label>
            <Input
              id="edit-target-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-target-url">Webhook URL *</Label>
            <Input
              id="edit-target-url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-target-format">Format *</Label>
            <Select value={format} onValueChange={(v) => setFormat(v as WebhookFormat)}>
              <SelectTrigger id="edit-target-format">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="discord">Discord</SelectItem>
                <SelectItem value="slack">Slack</SelectItem>
                <SelectItem value="generic_json">Generic JSON</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="edit-target-secret">Authorization Header</Label>
            <Input
              id="edit-target-secret"
              value={secretHeader}
              onChange={(e) => setSecretHeader(e.target.value)}
              placeholder="Optional — e.g. Bearer token123"
            />
          </div>

          <div className="flex items-center justify-between">
            <Label htmlFor="edit-target-enabled">Enabled</Label>
            <Switch id="edit-target-enabled" checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={updateTarget.isPending}>
              {updateTarget.isPending ? "Saving..." : "Save"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
