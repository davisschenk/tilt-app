import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";

export default function BrewNew() {
  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="New Brew"
        description="Create a new brew."
      />
    </div>
  );
}
