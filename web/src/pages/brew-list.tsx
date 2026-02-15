import { useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { Plus } from "lucide-react";
import { format } from "date-fns";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useBrews } from "@/hooks/use-brews";
import type { BrewResponse, BrewStatus } from "@/types";

const STATUS_BADGE: Record<string, { variant: "default" | "secondary" | "outline"; label: string }> = {
  Active: { variant: "default", label: "Active" },
  Completed: { variant: "secondary", label: "Completed" },
  Archived: { variant: "outline", label: "Archived" },
};

const COLOR_MAP: Record<string, string> = {
  Red: "#E03131",
  Green: "#2F9E44",
  Black: "#495057",
  Purple: "#7048E8",
  Orange: "#E8590C",
  Blue: "#1971C2",
  Yellow: "#F08C00",
  Pink: "#D6336C",
};

export default function BrewList() {
  const [statusFilter, setStatusFilter] = useState<BrewStatus | undefined>(undefined);
  const { data: brews, isLoading } = useBrews(statusFilter);
  const navigate = useNavigate();

  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Brews"
        description="Manage your brews."
        actions={
          <Button asChild>
            <Link to="/brews/new">
              <Plus className="mr-2 h-4 w-4" />
              New Brew
            </Link>
          </Button>
        }
      />

      <Tabs
        value={statusFilter ?? "all"}
        onValueChange={(v) => setStatusFilter(v === "all" ? undefined : (v as BrewStatus))}
        className="mb-4"
      >
        <TabsList>
          <TabsTrigger value="all">All</TabsTrigger>
          <TabsTrigger value="Active">Active</TabsTrigger>
          <TabsTrigger value="Completed">Completed</TabsTrigger>
          <TabsTrigger value="Archived">Archived</TabsTrigger>
        </TabsList>
      </Tabs>

      {isLoading ? (
        <div className="space-y-3">
          {Array.from({ length: 5 }).map((_, i) => (
            <Skeleton key={i} className="h-12 w-full" />
          ))}
        </div>
      ) : brews && brews.length > 0 ? (
        <div className="rounded-md border">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead className="hidden sm:table-cell">Style</TableHead>
                <TableHead className="hidden md:table-cell">Hydrometer</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="hidden lg:table-cell">OG</TableHead>
                <TableHead className="hidden lg:table-cell">Current SG</TableHead>
                <TableHead className="hidden lg:table-cell">ABV</TableHead>
                <TableHead className="hidden sm:table-cell">Start Date</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {brews.map((brew: BrewResponse) => {
                const badge = STATUS_BADGE[brew.status] ?? STATUS_BADGE.Active;
                const color = brew.latestReading?.color;
                return (
                  <TableRow
                    key={brew.id}
                    className="cursor-pointer"
                    onClick={() => navigate(`/brews/${brew.id}`)}
                  >
                    <TableCell className="font-medium">{brew.name}</TableCell>
                    <TableCell className="hidden sm:table-cell">
                      {brew.style ?? "—"}
                    </TableCell>
                    <TableCell className="hidden md:table-cell">
                      <div className="flex items-center gap-2">
                        {color && (
                          <span
                            className="inline-block h-3 w-3 rounded-full"
                            style={{ backgroundColor: COLOR_MAP[color] ?? "#868E96" }}
                          />
                        )}
                        <span>{color ?? "—"}</span>
                      </div>
                    </TableCell>
                    <TableCell>
                      <Badge variant={badge.variant}>{badge.label}</Badge>
                    </TableCell>
                    <TableCell className="hidden lg:table-cell">
                      {brew.og?.toFixed(3) ?? "—"}
                    </TableCell>
                    <TableCell className="hidden lg:table-cell">
                      {brew.latestReading?.gravity.toFixed(3) ?? "—"}
                    </TableCell>
                    <TableCell className="hidden lg:table-cell">
                      {brew.abv != null ? `${brew.abv.toFixed(1)}%` : "—"}
                    </TableCell>
                    <TableCell className="hidden sm:table-cell">
                      {brew.startDate
                        ? format(new Date(brew.startDate), "MMM d, yyyy")
                        : "—"}
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </div>
      ) : (
        <div className="flex flex-col items-center justify-center rounded-md border border-dashed p-12 text-center">
          <p className="text-muted-foreground mb-4">No brews found</p>
          <Button asChild>
            <Link to="/brews/new">
              <Plus className="mr-2 h-4 w-4" />
              Start a Brew
            </Link>
          </Button>
        </div>
      )}
    </div>
  );
}
