import { Routes, Route } from "react-router-dom";
import AppShell from "@/components/layout/app-shell";
import ProtectedRoute from "@/components/auth/protected-route";
import Dashboard from "@/pages/dashboard";
import BrewList from "@/pages/brew-list";
import BrewDetail from "@/pages/brew-detail";
import BrewNew from "@/pages/brew-new";
import HydrometerList from "@/pages/hydrometer-list";
import ApiKeysPage from "@/pages/api-keys";
import NotFound from "@/pages/not-found";

export default function App() {
  return (
    <ProtectedRoute>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/brews" element={<BrewList />} />
          <Route path="/brews/new" element={<BrewNew />} />
          <Route path="/brews/:id" element={<BrewDetail />} />
          <Route path="/hydrometers" element={<HydrometerList />} />
          <Route path="/settings/api-keys" element={<ApiKeysPage />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Routes>
    </ProtectedRoute>
  );
}
