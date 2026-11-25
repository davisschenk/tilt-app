import { useState } from "react";
import { Download, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import PageHeader from "@/components/layout/page-header";
import { LegalFooterLinks } from "@/components/legal-layout";
import { apiPost, downloadFile } from "@/lib/api";
import { useAuth } from "@/lib/auth-context";
import * as toast from "@/lib/toast";
import { useUnits, type Preferences } from "@/lib/units";

export default function AccountPage() {
  const { user } = useAuth();
  const { units, save } = useUnits();
  const [exporting, setExporting] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [confirmEmail, setConfirmEmail] = useState("");
  const [deleting, setDeleting] = useState(false);

  const emailMatches =
    !!user && confirmEmail.trim().toLowerCase() === user.email.trim().toLowerCase();

  function savePreferences(next: Preferences) {
    save.mutate(next, { onError: () => toast.error("Could not save preferences") });
  }

  async function handleExport() {
    setExporting(true);
    try {
      await downloadFile("/account/export", "tiltbrew-export.json");
      toast.success("Export downloaded");
    } catch {
      toast.error("Export failed");
    } finally {
      setExporting(false);
    }
  }

  async function handleDelete() {
    setDeleting(true);
    try {
      await apiPost("/account/delete", { confirmEmail });
      window.location.href = "/";
    } catch {
      toast.error("Could not delete the account");
      setDeleting(false);
    }
  }

  return (
    <div>
      <PageHeader title="Account" description="Your profile and your data." />

      <div className="space-y-6 max-w-2xl">
        <Card>
          <CardHeader>
            <CardTitle>Measurement preferences</CardTitle>
            <CardDescription>Used across your devices, charts, forms and alerts.</CardDescription>
          </CardHeader>
          <CardContent className="flex gap-6">
            <div className="space-y-2">
              <Label htmlFor="temperature-unit">Temperature</Label>
              <Select
                value={units.temperature}
                disabled={save.isPending}
                onValueChange={(v) => savePreferences({ ...units, temperature: v as Preferences["temperature"] })}
              >
                <SelectTrigger id="temperature-unit" aria-label="Temperature unit">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="F">°F</SelectItem>
                  <SelectItem value="C">°C</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label htmlFor="gravity-unit">Gravity</Label>
              <Select
                value={units.gravity}
                disabled={save.isPending}
                onValueChange={(v) => savePreferences({ ...units, gravity: v as Preferences["gravity"] })}
              >
                <SelectTrigger id="gravity-unit" aria-label="Gravity unit">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="SG">SG</SelectItem>
                  <SelectItem value="P">°P</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Profile</CardTitle>
            <CardDescription>Provided by your sign-in provider.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-1 text-sm">
            <p>
              <span className="text-muted-foreground">Name: </span>
              {user?.name}
            </p>
            <p>
              <span className="text-muted-foreground">Email: </span>
              {user?.email}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Export your data</CardTitle>
            <CardDescription>
              Download your hydrometers, brews, readings, log entries and alert settings as a JSON
              file. Secrets such as API key hashes are not included.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button variant="outline" onClick={handleExport} disabled={exporting}>
              <Download className="mr-2 h-4 w-4" />
              {exporting ? "Preparing..." : "Download export"}
            </Button>
          </CardContent>
        </Card>

        <Card className="border-destructive/50">
          <CardHeader>
            <CardTitle>Delete account</CardTitle>
            <CardDescription>
              Permanently delete your account and everything in it, including readings and uploaded
              photos. This cannot be undone.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <Button variant="destructive" onClick={() => setConfirmOpen(true)}>
              <Trash2 className="mr-2 h-4 w-4" />
              Delete my account
            </Button>
          </CardContent>
        </Card>

        <LegalFooterLinks />
      </div>

      <AlertDialog
        open={confirmOpen}
        onOpenChange={(open) => {
          setConfirmOpen(open);
          if (!open) setConfirmEmail("");
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete your account?</AlertDialogTitle>
            <AlertDialogDescription>
              All of your brews, readings, devices, alerts and photos will be deleted for good.
              Consider downloading an export first.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="space-y-2">
            <Label htmlFor="confirm-email">
              Type <span className="font-mono">{user?.email}</span> to confirm
            </Label>
            <Input
              id="confirm-email"
              value={confirmEmail}
              onChange={(e) => setConfirmEmail(e.target.value)}
              autoComplete="off"
            />
          </div>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <Button variant="destructive" disabled={!emailMatches || deleting} onClick={handleDelete}>
              {deleting ? "Deleting..." : "Delete everything"}
            </Button>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
