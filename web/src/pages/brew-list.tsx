import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";

export default function BrewList() {
  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Brews"
        description="Manage your brews."
      />
    </div>
  );
}
