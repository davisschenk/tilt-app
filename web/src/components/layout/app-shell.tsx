import { useState } from "react";
import { NavLink, Outlet } from "react-router-dom";
import { LayoutDashboard, Beer, Thermometer, Menu } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "@/components/ui/sheet";
import { Toaster } from "@/components/ui/sonner";
import ThemeToggle from "@/components/theme-toggle";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/brews", label: "Brews", icon: Beer },
  { to: "/hydrometers", label: "Hydrometers", icon: Thermometer },
];

function NavLinks({ onClick }: { onClick?: () => void }) {
  return (
    <>
      {navItems.map((item) => (
        <NavLink
          key={item.to}
          to={item.to}
          end={item.to === "/"}
          onClick={onClick}
          className={({ isActive }) =>
            cn(
              "flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors",
              isActive
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )
          }
        >
          <item.icon className="h-4 w-4" />
          {item.label}
        </NavLink>
      ))}
    </>
  );
}

export default function AppShell() {
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <div className="flex min-h-screen">
      <aside className="hidden md:flex w-64 flex-col border-r bg-muted/40">
        <div className="flex h-14 items-center border-b px-4">
          <h1 className="text-lg font-semibold">Tilt Dashboard</h1>
        </div>
        <nav className="flex-1 space-y-1 p-2">
          <NavLinks />
        </nav>
        <div className="flex items-center justify-between border-t p-4 text-xs text-muted-foreground">
          v0.1.0
          <ThemeToggle />
        </div>
      </aside>

      <div className="flex flex-1 flex-col">
        <header className="flex md:hidden h-14 items-center border-b px-4 gap-4">
          <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon">
                <Menu className="h-5 w-5" />
                <span className="sr-only">Toggle navigation</span>
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-64 p-0">
              <SheetHeader className="h-14 flex justify-center border-b px-4">
                <SheetTitle className="text-lg font-semibold">Tilt Dashboard</SheetTitle>
              </SheetHeader>
              <nav className="space-y-1 p-2">
                <NavLinks onClick={() => setMobileOpen(false)} />
              </nav>
            </SheetContent>
          </Sheet>
          <h1 className="flex-1 text-lg font-semibold">Tilt Dashboard</h1>
          <ThemeToggle />
        </header>

        <main className="flex-1 overflow-y-auto p-6">
          <Outlet />
        </main>
      </div>
      <Toaster />
    </div>
  );
}
