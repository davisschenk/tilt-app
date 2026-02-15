import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";

export default function HydrometerList() {
  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Hydrometers"
        description="Manage your Tilt hydrometers."
      />
    </div>
  );
}
