import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useDeleteNutrientSchedule } from "@/hooks/use-nutrient-schedule";
import type { NutrientScheduleResponse, NutrientProtocol } from "@/types";
import { Beaker, Trash2, AlertTriangle } from "lucide-react";
import { format } from "date-fns";

interface NutrientScheduleProps {
  brewId: string;
  schedule: NutrientScheduleResponse;
}

const PROTOCOL_LABELS: Record<NutrientProtocol, string> = {
  fermaid_o: "Fermaid O",
  fermaid_ok: "O + K",
  fermaid_okdap: "O + K + DAP",
};

export function NutrientSchedule({ brewId, schedule }: NutrientScheduleProps) {
  const deleteSchedule = useDeleteNutrientSchedule(brewId);

  const showK = schedule.nutrientProtocol !== "fermaid_o";
  const showDap = schedule.nutrientProtocol === "fermaid_okdap";

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle className="flex items-center gap-2">
          <Beaker className="h-5 w-5" />
          Nutrient Schedule
        </CardTitle>
        <div className="flex items-center gap-2">
          <Badge variant="outline">
            {PROTOCOL_LABELS[schedule.nutrientProtocol]}
          </Badge>
          <Button
            variant="ghost"
            size="sm"
            className="text-destructive"
            onClick={() => {
              if (confirm("Remove this nutrient schedule?")) {
                deleteSchedule.mutate();
              }
            }}
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {schedule.maxDosageCapped && (
          <div className="bg-yellow-50 dark:bg-yellow-950 border border-yellow-200 dark:border-yellow-800 rounded-md p-3 flex items-center gap-2 text-sm">
            <AlertTriangle className="h-4 w-4 text-yellow-600" />
            <span>Nutrient amounts capped at max dosage limits.</span>
          </div>
        )}

        {/* Summary Stats */}
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
          <div>
            <span className="text-muted-foreground">Target YAN</span>
            <p className="font-medium">
              {schedule.totalYanPpm.toFixed(0)} ppm
            </p>
          </div>
          <div>
            <span className="text-muted-foreground">Effective YAN</span>
            <p className="font-medium">
              {schedule.effectiveYanPpm.toFixed(0)} ppm
            </p>
          </div>
          <div>
            <span className="text-muted-foreground">1/3 Sugar Break</span>
            <p className="font-medium">
              {schedule.oneThirdBreakSg.toFixed(3)}
            </p>
          </div>
          <div>
            <span className="text-muted-foreground">OG / Volume</span>
            <p className="font-medium">
              {schedule.og.toFixed(3)} / {schedule.batchSizeGallons} gal
            </p>
          </div>
        </div>

        <div className="grid grid-cols-3 gap-3 text-sm">
          <div>
            <span className="text-muted-foreground">Yeast</span>
            <p className="font-medium">{schedule.yeastGrams}g</p>
          </div>
          <div>
            <span className="text-muted-foreground">GoFerm</span>
            <p className="font-medium">{schedule.goFermGrams}g</p>
          </div>
          <div>
            <span className="text-muted-foreground">Rehydration Water</span>
            <p className="font-medium">{schedule.rehydrationWaterMl}mL</p>
          </div>
        </div>

        {/* Addition Timeline */}
        <div className="space-y-2">
          {schedule.additions.map((a) => {
            const isNotified = !!a.notifiedAt;
            return (
              <div
                key={a.id}
                className="flex items-center gap-3 rounded-md border p-3"
              >
                <div
                  className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-medium ${
                    isNotified
                      ? "bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-400"
                      : "bg-muted text-muted-foreground"
                  }`}
                >
                  {a.additionNumber}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2 flex-wrap">
                    {a.fermaidOGrams > 0 && (
                      <span className="text-sm font-medium">
                        {a.fermaidOGrams.toFixed(1)}g Fermaid O
                      </span>
                    )}
                    {showK && a.fermaidKGrams > 0 && (
                      <span className="text-sm font-medium">
                        {a.fermaidKGrams.toFixed(1)}g Fermaid K
                      </span>
                    )}
                    {showDap && a.dapGrams > 0 && (
                      <span className="text-sm font-medium">
                        {a.dapGrams.toFixed(1)}g DAP
                      </span>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {a.triggerType === "time"
                      ? `${a.targetHours}h after pitch`
                      : `SG ≤ ${a.targetGravity?.toFixed(3)} or day 7`}
                  </p>
                </div>
                <div>
                  {isNotified ? (
                    <Badge className="bg-green-100 text-green-700 dark:bg-green-950 dark:text-green-400 border-green-200 dark:border-green-800">
                      Notified {format(new Date(a.notifiedAt!), "MMM d, h:mm a")}
                    </Badge>
                  ) : (
                    <Badge variant="secondary">Pending</Badge>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
