import { useState, useEffect } from "react";
import { format } from "date-fns";
import {
  Beaker,
  Hop,
  CheckCircle2,
  Thermometer,
  Snowflake,
  Droplets,
  ArrowRightLeft,
  Package,
  TestTube,
  Star,
  SlidersHorizontal,
  StickyNote,
  Plus,
  Pencil,
  Trash2,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
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
import { useBrewEvents, useCreateBrewEvent, useUpdateBrewEvent, useDeleteBrewEvent } from "@/hooks/use-brew-events";
import * as toast from "@/lib/toast";
import type { BrewEventResponse, BrewEventType, CreateBrewEvent, UpdateBrewEvent } from "@/types";

const EVENT_ICONS: Record<BrewEventType, React.ReactNode> = {
  yeast_pitch: <Beaker className="h-4 w-4" />,
  dry_hop: <Hop className="h-4 w-4" />,
  fermentation_complete: <CheckCircle2 className="h-4 w-4" />,
  diacetyl_rest: <Thermometer className="h-4 w-4" />,
  cold_crash: <Snowflake className="h-4 w-4" />,
  fining_addition: <Droplets className="h-4 w-4" />,
  transfer: <ArrowRightLeft className="h-4 w-4" />,
  packaged: <Package className="h-4 w-4" />,
  gravity_sample: <TestTube className="h-4 w-4" />,
  tasting_note: <Star className="h-4 w-4" />,
  temperature_change: <SlidersHorizontal className="h-4 w-4" />,
  note: <StickyNote className="h-4 w-4" />,
  nutrient_addition: <Droplets className="h-4 w-4" />,
};

const EVENT_COLORS: Record<BrewEventType, string> = {
  yeast_pitch: "text-green-600",
  dry_hop: "text-orange-500",
  fermentation_complete: "text-emerald-600",
  diacetyl_rest: "text-yellow-600",
  cold_crash: "text-cyan-500",
  fining_addition: "text-pink-500",
  transfer: "text-blue-500",
  packaged: "text-violet-600",
  gravity_sample: "text-teal-600",
  tasting_note: "text-amber-500",
  temperature_change: "text-purple-500",
  note: "text-gray-500",
  nutrient_addition: "text-lime-600",
};

const EVENT_TYPE_LABELS: Record<BrewEventType, string> = {
  yeast_pitch: "Yeast Pitch",
  dry_hop: "Dry Hop",
  fermentation_complete: "Fermentation Complete",
  diacetyl_rest: "Diacetyl Rest",
  cold_crash: "Cold Crash",
  fining_addition: "Fining Addition",
  transfer: "Transfer",
  packaged: "Packaged",
  gravity_sample: "Gravity Sample",
  tasting_note: "Tasting Note",
  temperature_change: "Temperature Change",
  note: "Note",
  nutrient_addition: "Nutrient Addition",
};

const ALL_EVENT_TYPES: BrewEventType[] = [
  "yeast_pitch",
  "dry_hop",
  "fermentation_complete",
  "diacetyl_rest",
  "cold_crash",
  "fining_addition",
  "transfer",
  "packaged",
  "gravity_sample",
  "tasting_note",
  "temperature_change",
  "note",
];

function toDatetimeLocal(iso: string): string {
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

function nowDatetimeLocal(): string {
  return toDatetimeLocal(new Date().toISOString());
}

export interface CreateEventDialogProps {
  brewId: string;
  open: boolean;
  onOpenChange: (o: boolean) => void;
  initialEventTime?: string;
}

export function CreateEventDialog({ brewId, open, onOpenChange, initialEventTime }: CreateEventDialogProps) {
  const create = useCreateBrewEvent(brewId);
  const [eventType, setEventType] = useState<BrewEventType>("note");
  const [label, setLabel] = useState("");
  const [notes, setNotes] = useState("");
  const [gravity, setGravity] = useState("");
  const [temp, setTemp] = useState("");
  const [eventTime, setEventTime] = useState(nowDatetimeLocal());

  useEffect(() => {
    if (open) {
      setEventTime(initialEventTime ? toDatetimeLocal(initialEventTime) : nowDatetimeLocal());
    }
  }, [open, initialEventTime]);

  function reset() {
    setEventType("note");
    setLabel("");
    setNotes("");
    setGravity("");
    setTemp("");
    setEventTime(nowDatetimeLocal());
  }

  function handleSubmit() {
    const payload: CreateBrewEvent = {
      brewId,
      eventType,
      label: label || EVENT_TYPE_LABELS[eventType],
      notes: notes || null,
      gravityAtEvent: gravity ? parseFloat(gravity) : null,
      tempAtEvent: temp ? parseFloat(temp) : null,
      eventTime: new Date(eventTime).toISOString(),
    };
    create.mutate(payload, {
      onSuccess: () => {
        toast.success("Event added");
        reset();
        onOpenChange(false);
      },
      onError: () => toast.error("Failed to add event"),
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Add Fermentation Event</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label>Event Type</Label>
            <Select value={eventType} onValueChange={(v) => setEventType(v as BrewEventType)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {ALL_EVENT_TYPES.map((t) => (
                  <SelectItem key={t} value={t}>
                    <span className="flex items-center gap-2">
                      <span className={EVENT_COLORS[t]}>{EVENT_ICONS[t]}</span>
                      {EVENT_TYPE_LABELS[t]}
                    </span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1.5">
            <Label>Label</Label>
            <Input
              placeholder={EVENT_TYPE_LABELS[eventType]}
              value={label}
              onChange={(e) => setLabel(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label>Notes</Label>
            <Textarea
              placeholder="Optional notes..."
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              rows={3}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label>Gravity at Event</Label>
              <Input
                type="number"
                step="0.001"
                placeholder="1.012"
                value={gravity}
                onChange={(e) => setGravity(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Temp (°F)</Label>
              <Input
                type="number"
                step="0.1"
                placeholder="68.0"
                value={temp}
                onChange={(e) => setTemp(e.target.value)}
              />
            </div>
          </div>
          <div className="space-y-1.5">
            <Label>Event Time</Label>
            <Input
              type="datetime-local"
              value={eventTime}
              onChange={(e) => setEventTime(e.target.value)}
            />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={create.isPending}>Add Event</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface EditEventDialogProps {
  brewId: string;
  event: BrewEventResponse;
  open: boolean;
  onOpenChange: (o: boolean) => void;
}

function EditEventDialog({ brewId, event, open, onOpenChange }: EditEventDialogProps) {
  const update = useUpdateBrewEvent(brewId, event.id);
  const [label, setLabel] = useState(event.label);
  const [notes, setNotes] = useState(event.notes ?? "");
  const [gravity, setGravity] = useState(event.gravityAtEvent?.toString() ?? "");
  const [temp, setTemp] = useState(event.tempAtEvent?.toString() ?? "");
  const [eventTime, setEventTime] = useState(toDatetimeLocal(event.eventTime));

  function handleSubmit() {
    const payload: UpdateBrewEvent = {
      label,
      notes: notes || null,
      gravityAtEvent: gravity ? parseFloat(gravity) : null,
      tempAtEvent: temp ? parseFloat(temp) : null,
      eventTime: new Date(eventTime).toISOString(),
    };
    update.mutate(payload, {
      onSuccess: () => {
        toast.success("Event updated");
        onOpenChange(false);
      },
      onError: () => toast.error("Failed to update event"),
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Edit Event</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label>Label</Label>
            <Input value={label} onChange={(e) => setLabel(e.target.value)} />
          </div>
          <div className="space-y-1.5">
            <Label>Notes</Label>
            <Textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label>Gravity</Label>
              <Input type="number" step="0.001" value={gravity} onChange={(e) => setGravity(e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label>Temp (°F)</Label>
              <Input type="number" step="0.1" value={temp} onChange={(e) => setTemp(e.target.value)} />
            </div>
          </div>
          <div className="space-y-1.5">
            <Label>Event Time</Label>
            <Input type="datetime-local" value={eventTime} onChange={(e) => setEventTime(e.target.value)} />
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={update.isPending}>Save Changes</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface DeleteEventDialogProps {
  brewId: string;
  event: BrewEventResponse;
  open: boolean;
  onOpenChange: (o: boolean) => void;
}

function DeleteEventDialog({ brewId, event, open, onOpenChange }: DeleteEventDialogProps) {
  const del = useDeleteBrewEvent(brewId);

  function handleDelete() {
    del.mutate(event.id, {
      onSuccess: () => {
        toast.success("Event deleted");
        onOpenChange(false);
      },
      onError: () => toast.error("Failed to delete event"),
    });
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>Delete Event</DialogTitle>
        </DialogHeader>
        <p className="text-sm text-muted-foreground">
          Delete <span className="font-medium text-foreground">"{event.label}"</span>? This cannot be undone.
        </p>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>Cancel</Button>
          <Button variant="destructive" onClick={handleDelete} disabled={del.isPending}>Delete</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface BrewEventLogProps {
  brewId: string;
}

export default function BrewEventLog({ brewId }: BrewEventLogProps) {
  const { data: events, isLoading } = useBrewEvents(brewId);
  const [expanded, setExpanded] = useState(true);
  const [addOpen, setAddOpen] = useState(false);
  const [editEvent, setEditEvent] = useState<BrewEventResponse | null>(null);
  const [deleteEvent, setDeleteEvent] = useState<BrewEventResponse | null>(null);

  const sorted = events?.slice().sort((a, b) =>
    new Date(a.eventTime).getTime() - new Date(b.eventTime).getTime()
  ) ?? [];

  return (
    <div>
      <div className="flex items-center justify-between gap-2 mb-3">
        <button
          type="button"
          className="flex items-center gap-2 text-lg font-semibold hover:text-primary transition-colors min-w-0"
          onClick={() => setExpanded((e) => !e)}
        >
          <StickyNote className="h-5 w-5 shrink-0" />
          <span className="truncate">Fermentation Log</span>
          {events && events.length > 0 && (
            <Badge variant="secondary" className="ml-1 shrink-0">{events.length}</Badge>
          )}
          {expanded ? <ChevronUp className="h-4 w-4 shrink-0" /> : <ChevronDown className="h-4 w-4 shrink-0" />}
        </button>
        <Button variant="outline" size="sm" className="shrink-0" onClick={() => setAddOpen(true)}>
          <Plus className="mr-1 h-3 w-3" />
          Add Event
        </Button>
      </div>

      {expanded && (
        <div className="space-y-2">
          {isLoading ? (
            <p className="text-sm text-muted-foreground py-4">Loading events...</p>
          ) : sorted.length === 0 ? (
            <p className="text-sm text-muted-foreground py-4">No events logged yet. Add the first one!</p>
          ) : (
            sorted.map((ev) => (
              <Card key={ev.id} className="overflow-hidden">
                <CardContent className="py-3 flex items-start gap-3 min-w-0">
                  <span className={`mt-0.5 shrink-0 ${EVENT_COLORS[ev.eventType]}`}>
                    {EVENT_ICONS[ev.eventType]}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-medium text-sm">{ev.label}</span>
                      <Badge variant="outline" className="text-xs">
                        {EVENT_TYPE_LABELS[ev.eventType]}
                      </Badge>
                      {ev.gravityAtEvent != null && (
                        <span className="text-xs text-muted-foreground">SG {ev.gravityAtEvent.toFixed(3)}</span>
                      )}
                      {ev.tempAtEvent != null && (
                        <span className="text-xs text-muted-foreground">{ev.tempAtEvent.toFixed(1)}°F</span>
                      )}
                    </div>
                    {ev.notes && (
                      <p className="text-xs text-muted-foreground mt-0.5 truncate">{ev.notes}</p>
                    )}
                    <p className="text-xs text-muted-foreground mt-0.5">
                      {format(new Date(ev.eventTime), "MMM d, yyyy 'at' HH:mm")}
                    </p>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7"
                      onClick={() => setEditEvent(ev)}
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-destructive hover:text-destructive"
                      onClick={() => setDeleteEvent(ev)}
                    >
                      <Trash2 className="h-3 w-3" />
                    </Button>
                  </div>
                </CardContent>
              </Card>
            ))
          )}
        </div>
      )}

      <CreateEventDialog brewId={brewId} open={addOpen} onOpenChange={setAddOpen} />
      {editEvent && (
        <EditEventDialog
          brewId={brewId}
          event={editEvent}
          open={!!editEvent}
          onOpenChange={(o) => { if (!o) setEditEvent(null); }}
        />
      )}
      {deleteEvent && (
        <DeleteEventDialog
          brewId={brewId}
          event={deleteEvent}
          open={!!deleteEvent}
          onOpenChange={(o) => { if (!o) setDeleteEvent(null); }}
        />
      )}
    </div>
  );
}
