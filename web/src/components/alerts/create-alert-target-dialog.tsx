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
import { useCreateAlertTarget } from "@/hooks/use-alert-targets";
import * as toast from "@/lib/toast";
import type { WebhookFormat } from "@/types";

interface CreateAlertTargetDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export default function CreateAlertTargetDialog({
  open,
  onOpenChange,
}: CreateAlertTargetDialogProps) {
  const createTarget = useCreateAlertTarget();

  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [format, setFormat] = useState<WebhookFormat>("generic_json");
  const [secretHeader, setSecretHeader] = useState("");
  const [enabled, setEnabled] = useState(true);
  const [error, setError] = useState("");

  function resetForm() {
    setName("");
    setUrl("");
    setFormat("generic_json");
    setSecretHeader("");
    setEnabled(true);
    setError("");
  }

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

    createTarget.mutate(
      {
        name: name.trim(),
        url: url.trim(),
        format,
        secretHeader: secretHeader.trim() || null,
        enabled,
      },
      {
        onSuccess: () => {
          toast.success("Alert target created");
          resetForm();
          onOpenChange(false);
        },
        onError: () => {
          toast.error("Failed to create alert target");
        },
      },
    );
  }

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) resetForm(); onOpenChange(o); }}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Add Alert Target</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="target-name">Name *</Label>
            <Input
              id="target-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Discord Brew Alerts"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="target-url">Webhook URL *</Label>
            <Input
              id="target-url"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              placeholder="https://discord.com/api/webhooks/..."
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="target-format">Format *</Label>
            <Select value={format} onValueChange={(v) => setFormat(v as WebhookFormat)}>
              <SelectTrigger id="target-format">
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
            <Label htmlFor="target-secret">Authorization Header</Label>
            <Input
              id="target-secret"
              value={secretHeader}
              onChange={(e) => setSecretHeader(e.target.value)}
              placeholder="Optional — e.g. Bearer token123"
            />
          </div>

          <div className="flex items-center justify-between">
            <Label htmlFor="target-enabled">Enabled</Label>
            <Switch id="target-enabled" checked={enabled} onCheckedChange={setEnabled} />
          </div>

          {error && <p className="text-sm text-destructive">{error}</p>}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => { resetForm(); onOpenChange(false); }}>
              Cancel
            </Button>
            <Button type="submit" disabled={createTarget.isPending}>
              {createTarget.isPending ? "Creating..." : "Create"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
