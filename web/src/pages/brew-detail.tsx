import { useParams } from "react-router-dom";

export default function BrewDetail() {
  const { id } = useParams<{ id: string }>();
  return (
    <div>
      <h1 className="text-2xl font-bold">Brew Detail</h1>
      <p className="text-muted-foreground">Brew ID: {id}</p>
    </div>
  );
}
