import { Link, useLocation } from "react-router-dom";
import { ChevronRight, House } from "lucide-react";

const routeLabels: Record<string, string> = {
  brews: "Brews",
  new: "New Brew",
  hydrometers: "Hydrometers",
};

export default function Breadcrumbs() {
  const { pathname } = useLocation();
  const segments = pathname.split("/").filter(Boolean);

  if (segments.length === 0) return null;

  return (
    <nav aria-label="Breadcrumb" className="flex items-center gap-1 text-sm text-muted-foreground mb-4">
      <Link to="/" aria-label="Home" className="hover:text-foreground transition-colors">
        <House className="h-4 w-4" aria-hidden="true" />
      </Link>
      {segments.map((segment, index) => {
        const path = "/" + segments.slice(0, index + 1).join("/");
        const isLast = index === segments.length - 1;
        const label = routeLabels[segment] ?? segment;

        return (
          <span key={path} className="flex items-center gap-1">
            <ChevronRight className="h-3 w-3" />
            {isLast ? (
              <span className="text-foreground font-medium">{label}</span>
            ) : (
              <Link to={path} className="hover:text-foreground transition-colors">
                {label}
              </Link>
            )}
          </span>
        );
      })}
    </nav>
  );
}
