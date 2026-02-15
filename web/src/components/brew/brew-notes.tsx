import { useState } from "react";
import MDEditor from "@uiw/react-md-editor";
import { Pencil, Check, X } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useUpdateBrew } from "@/hooks/use-brews";
import { useTheme } from "@/components/theme-provider";
import * as toast from "@/lib/toast";

interface BrewNotesProps {
  brewId: string;
  notes: string | null;
}

export default function BrewNotes({ brewId, notes }: BrewNotesProps) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(notes ?? "");
  const updateBrew = useUpdateBrew(brewId);
  const { theme } = useTheme();

  const colorMode =
    theme === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? "dark"
        : "light"
      : theme;

  function handleSave() {
    updateBrew.mutate(
      { notes: draft.trim() },
      {
        onSuccess: () => {
          toast.success("Notes saved");
          setEditing(false);
        },
        onError: () => toast.error("Failed to save notes"),
      },
    );
  }

  function handleCancel() {
    setDraft(notes ?? "");
    setEditing(false);
  }

  return (
    <Card className="mt-6">
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-base">Notes</CardTitle>
        {!editing && (
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => {
              setDraft(notes ?? "");
              setEditing(true);
            }}
          >
            <Pencil className="h-4 w-4" />
          </Button>
        )}
      </CardHeader>
      <CardContent>
        {editing ? (
          <div className="space-y-3">
            <div data-color-mode={colorMode}>
              <MDEditor
                value={draft}
                onChange={(val) => setDraft(val ?? "")}
                preview="live"
                height={300}
              />
            </div>
            <div className="flex gap-2">
              <Button size="sm" onClick={handleSave} disabled={updateBrew.isPending}>
                <Check className="mr-1 h-3 w-3" />
                Save
              </Button>
              <Button size="sm" variant="outline" onClick={handleCancel}>
                <X className="mr-1 h-3 w-3" />
                Cancel
              </Button>
            </div>
          </div>
        ) : notes ? (
          <div data-color-mode={colorMode}>
            <MDEditor.Markdown source={notes} />
          </div>
        ) : (
          <p
            className="text-sm text-muted-foreground cursor-pointer hover:text-foreground"
            onClick={() => {
              setDraft("");
              setEditing(true);
            }}
          >
            Click to add notes...
          </p>
        )}
      </CardContent>
    </Card>
  );
}
