import { useParams } from "react-router-dom";
import Breadcrumbs from "@/components/layout/breadcrumbs";
import PageHeader from "@/components/layout/page-header";

export default function BrewDetail() {
  const { id } = useParams<{ id: string }>();
  return (
    <div>
      <Breadcrumbs />
      <PageHeader
        title="Brew Detail"
        description={`Brew ID: ${id}`}
      />
    </div>
  );
}
