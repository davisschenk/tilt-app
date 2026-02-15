import { Beer, Thermometer, Activity, BarChart3 } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import PageHeader from "@/components/layout/page-header";
import { useBrews } from "@/hooks/use-brews";
import { useHydrometers } from "@/hooks/use-hydrometers";
import { useReadings } from "@/hooks/use-readings";

function StatCard({
  title,
  value,
  description,
  icon: Icon,
  isLoading,
}: {
  title: string;
  value: string;
  description?: string;
  icon: React.ComponentType<{ className?: string }>;
  isLoading: boolean;
}) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">{title}</CardTitle>
        <Icon className="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <Skeleton className="h-8 w-20" />
        ) : (
          <div className="text-2xl font-bold">{value}</div>
        )}
        {description && (
          <p className="text-xs text-muted-foreground mt-1">{description}</p>
        )}
      </CardContent>
    </Card>
  );
}

export default function Dashboard() {
  const { data: activeBrews, isLoading: brewsLoading } = useBrews("Active");
  const { data: hydrometers, isLoading: hydrometersLoading } = useHydrometers();
  const { data: readings, isLoading: readingsLoading } = useReadings({ limit: 1 });

  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const latestReading = readings?.[0];

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description="Overview of your brewing activity."
      />
      <div className="grid gap-4 sm:grid-cols-2">
        <StatCard
          title="Active Brews"
          value={String(activeBrews?.length ?? 0)}
          icon={Beer}
          isLoading={brewsLoading}
        />
        <StatCard
          title="Total Hydrometers"
          value={String(hydrometers?.length ?? 0)}
          icon={Thermometer}
          isLoading={hydrometersLoading}
        />
        <StatCard
          title="Latest Reading"
          value={
            latestReading
              ? `${latestReading.gravity.toFixed(3)} SG / ${latestReading.temperatureF}°F`
              : "—"
          }
          description={latestReading ? `${latestReading.color} hydrometer` : "No readings yet"}
          icon={Activity}
          isLoading={readingsLoading}
        />
        <StatCard
          title="Readings Today"
          value="—"
          description="From all hydrometers"
          icon={BarChart3}
          isLoading={readingsLoading}
        />
      </div>
    </div>
  );
}
