import { useState, useMemo, useEffect } from "react";
import { format } from "date-fns";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useReadings } from "@/hooks/use-readings";

const PAGE_SIZE = 25;

interface ReadingsTableProps {
  brewId: string;
}

export default function ReadingsTable({ brewId }: ReadingsTableProps) {
  const [page, setPage] = useState(0);
  const { data: readings, isLoading } = useReadings({ brewId });

  useEffect(() => {
    setPage(0);
  }, [brewId]);

  const sorted = useMemo(() => {
    if (!readings) return [];
    return readings
      .slice()
      .sort((a, b) => new Date(b.recordedAt).getTime() - new Date(a.recordedAt).getTime());
  }, [readings]);

  const totalPages = Math.max(1, Math.ceil(sorted.length / PAGE_SIZE));
  const pageData = sorted.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);

  return (
    <Card className="mt-6">
      <CardHeader>
        <CardTitle className="text-base">Readings Data</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-10 w-full" />
            ))}
          </div>
        ) : sorted.length === 0 ? (
          <div className="flex items-center justify-center h-32 text-muted-foreground">
            No readings recorded yet
          </div>
        ) : (
          <>
            <div className="rounded-md border">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Recorded At</TableHead>
                    <TableHead>Temperature (°F)</TableHead>
                    <TableHead>Gravity (SG)</TableHead>
                    <TableHead>RSSI</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {pageData.map((r) => (
                    <TableRow key={r.id}>
                      <TableCell>
                        {format(new Date(r.recordedAt), "MMM d, yyyy HH:mm:ss")}
                      </TableCell>
                      <TableCell>{r.temperatureF.toFixed(1)}</TableCell>
                      <TableCell>{r.gravity.toFixed(3)}</TableCell>
                      <TableCell>{r.rssi ?? "—"}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </div>

            <div className="flex items-center justify-between mt-4">
              <p className="text-sm text-muted-foreground">
                {sorted.length} readings · Page {page + 1} of {totalPages}
              </p>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPage((p) => Math.max(0, p - 1))}
                  disabled={page === 0}
                >
                  <ChevronLeft className="h-4 w-4 mr-1" />
                  Previous
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
                  disabled={page >= totalPages - 1}
                >
                  Next
                  <ChevronRight className="h-4 w-4 ml-1" />
                </Button>
              </div>
            </div>
          </>
        )}
      </CardContent>
    </Card>
  );
}
