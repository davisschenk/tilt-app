import { format } from "date-fns";
import { CheckCircle2, Clock, Droplets, AlertCircle } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { useNutrientSchedule } from "@/hooks/use-nutrient-schedule";
import { useBrewEvents } from "@/hooks/use-brew-events";
import type { BrewResponse, NutrientAddition, NutrientProduct } from "@/types";

interface Props {
  brew: BrewResponse;
}

const PRODUCT_LABELS: Record<NutrientProduct, string> = {
  fermaid_o: "Fermaid-O",
  fermaid_k: "Fermaid-K",
  dap: "DAP",
  go_ferm: "GoFerm",
};

const G_PER_TSP: Record<NutrientProduct, number> = {
  fermaid_o: 2.6,
  fermaid_k: 2.8,
  dap: 3.1,
  go_ferm: 2.0,
};

const PROTOCOL_DISPLAY: Record<string, string> = {
  tosna_2: "TOSNA 2.0",
  tosna_3: "TOSNA 3.0",
  advanced_sna: "Advanced SNA",
};

function gramsToTsp(product: NutrientProduct, grams: number): number {
  return grams / (G_PER_TSP[product] ?? 1);
}

function triggerLabel(addition: NutrientAddition): string {
  if (addition.primaryTrigger === "at_pitch") return "At pitch";
  if (addition.primaryTrigger === "gravity_threshold" && addition.gravityThreshold != null) {
    const fallback = addition.fallbackHours != null ? ` (fallback: ${addition.fallbackHours}h)` : "";
    return `Gravity ≤ ${addition.gravityThreshold.toFixed(3)}${fallback}`;
  }
  if (addition.fallbackHours != null) return `${addition.fallbackHours}h elapsed`;
  return "—";
}

function AdditionRow({
  addition,
  completed,
  current,
}: {
  addition: NutrientAddition;
  completed: boolean;
  current: boolean;
}) {
  const tsp = gramsToTsp(addition.product, addition.amountGrams);

  return (
    <div
      className={`flex items-start gap-4 rounded-lg border p-4 transition-colors ${
        completed
          ? "border-green-200 bg-green-50 dark:border-green-900 dark:bg-green-950/40"
          : current
            ? "border-blue-300 bg-blue-50 dark:border-blue-800 dark:bg-blue-950/40"
            : "border-border bg-card"
      }`}
    >
      <div className="mt-0.5 shrink-0">
        {completed ? (
          <CheckCircle2 className="h-5 w-5 text-green-600 dark:text-green-400" />
        ) : current ? (
          <Droplets className="h-5 w-5 text-blue-600 dark:text-blue-400" />
        ) : (
          <Clock className="h-5 w-5 text-muted-foreground" />
        )}
      </div>

      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="font-semibold text-sm">Addition #{addition.additionNumber}</span>
          <Badge variant={completed ? "secondary" : current ? "default" : "outline"} className="text-xs">
            {PRODUCT_LABELS[addition.product] ?? addition.product}
          </Badge>
          {completed && (
            <Badge variant="secondary" className="text-xs bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300">
              Done
            </Badge>
          )}
          {current && !completed && (
            <Badge className="text-xs bg-blue-600 text-white">Due now</Badge>
          )}
        </div>

        <p className="text-sm text-muted-foreground mt-1">{triggerLabel(addition)}</p>

        {addition.dueAt && (
          <p className="text-xs text-muted-foreground mt-0.5">
            Fallback: {format(new Date(addition.dueAt), "MMM d 'at' HH:mm")}
          </p>
        )}
      </div>

      <div className="text-right shrink-0">
        <p className="font-semibold text-sm">{addition.amountGrams.toFixed(1)} g</p>
        <p className="text-xs text-muted-foreground">{tsp.toFixed(1)} tsp</p>
      </div>
    </div>
  );
}

export default function NutrientScheduleTable({ brew }: Props) {
  const { data: schedule, isLoading, error } = useNutrientSchedule(brew.id);
  const { data: events } = useBrewEvents(brew.id);

  const isConfigured =
    brew.batchSizeGallons != null &&
    brew.pitchTime != null &&
    brew.og != null &&
    brew.targetFg != null;

  if (!isConfigured) {
    return null;
  }

  if (isLoading) {
    return (
      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="text-base">Nutrient Schedule</CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </CardContent>
      </Card>
    );
  }

  if (error || !schedule) {
    return (
      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="text-base">Nutrient Schedule</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <AlertCircle className="h-4 w-4" />
            <span>Could not load nutrient schedule. Ensure OG, Target FG, batch size, and pitch time are set.</span>
          </div>
        </CardContent>
      </Card>
    );
  }

  const completedNums = new Set(
    (events ?? [])
      .filter((e) => e.eventType === "nutrient_addition")
      .map((e) => {
        const match = e.notes?.match(/^Addition #(\d+)/);
        return match ? parseInt(match[1], 10) : null;
      })
      .filter((n): n is number => n != null),
  );

  const currentGravity = brew.latestReading?.gravity;

  function isCurrentlyDue(addition: NutrientAddition): boolean {
    if (completedNums.has(addition.additionNumber)) return false;
    if (addition.primaryTrigger === "at_pitch") return true;
    if (
      addition.gravityThreshold != null &&
      currentGravity != null &&
      currentGravity <= addition.gravityThreshold
    )
      return true;
    if (addition.dueAt != null && new Date() >= new Date(addition.dueAt)) return true;
    return false;
  }

  const protocolLabel =
    PROTOCOL_DISPLAY[schedule.protocol] ?? schedule.protocol;
  const completedCount = schedule.additions.filter((a) =>
    completedNums.has(a.additionNumber),
  ).length;

  const YAN_PER_G: Record<NutrientProduct, number> = {
    fermaid_o: 40,
    fermaid_k: 100,
    dap: 210,
    go_ferm: 0,
  };
  const batchLiters = schedule.batchSizeLiters;
  const providedYan = schedule.additions.reduce((acc, a) => {
    const yanPer = YAN_PER_G[a.product] ?? 0;
    return acc + (a.amountGrams * yanPer) / batchLiters;
  }, 0);

  return (
    <Card className="mt-6">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between flex-wrap gap-2">
          <CardTitle className="text-base">Nutrient Schedule — {protocolLabel}</CardTitle>
          <div className="flex flex-wrap items-center gap-4 text-sm">
            <span>
              <span className="font-medium">Target YAN:</span>{" "}
              {schedule.totalYanRequiredPpm.toFixed(0)} ppm
            </span>
            <span className="text-muted-foreground">|</span>
            <span>
              <span className="font-medium">Provided by schedule:</span>{" "}
              {providedYan.toFixed(0)} ppm
            </span>
            {schedule.resolvedFromStrain && (
              <span className="rounded-full bg-blue-100 px-2 py-0.5 text-xs text-blue-700 dark:bg-blue-900 dark:text-blue-200">
                Auto-detected from strain
              </span>
            )}
          </div>
          <div className="flex items-center gap-3 text-sm text-muted-foreground">
            <span>{schedule.batchSizeGallons.toFixed(1)} gal</span>
            <span>·</span>
            <span className="capitalize">{schedule.nitrogenRequirement} N</span>
            {completedCount > 0 && (
              <>
                <span>·</span>
                <span className="text-green-600 dark:text-green-400 font-medium">
                  {completedCount}/{schedule.additions.length} done
                </span>
              </>
            )}
          </div>
        </div>

        {Object.keys(schedule.nutrientTotals).length > 0 && (
          <div className="flex flex-wrap gap-3 mt-2">
            {Object.entries(schedule.nutrientTotals).map(([product, grams]) => {
              const typedProduct = product.replace("-", "_") as NutrientProduct;
              const tsp = grams / (G_PER_TSP[typedProduct] ?? 1);
              const label =
                PRODUCT_LABELS[typedProduct] ?? product;
              return (
                <div
                  key={product}
                  className="rounded-md bg-muted px-2.5 py-1 text-xs"
                >
                  <span className="font-medium">{label}</span>
                  <span className="text-muted-foreground ml-1">
                    {grams.toFixed(1)}g / {tsp.toFixed(1)} tsp total
                  </span>
                </div>
              );
            })}
          </div>
        )}
      </CardHeader>

      <CardContent className="space-y-3">
        {schedule.additions.map((addition) => (
          <AdditionRow
            key={addition.additionNumber}
            addition={addition}
            completed={completedNums.has(addition.additionNumber)}
            current={isCurrentlyDue(addition)}
          />
        ))}
      </CardContent>
    </Card>
  );
}
