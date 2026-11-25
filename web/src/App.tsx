import { Routes, Route } from "react-router-dom";
import AppShell from "@/components/layout/app-shell";
import ProtectedRoute from "@/components/auth/protected-route";
import BrewReport, { PublicBrewReport } from "@/pages/brew-report";
import Dashboard from "@/pages/dashboard";
import BrewList from "@/pages/brew-list";
import BrewDetail from "@/pages/brew-detail";
import BrewNew from "@/pages/brew-new";
import HydrometerList from "@/pages/hydrometer-list";
import DeviceSetup from "@/pages/device-setup";
import AlertTargets from "@/pages/alert-targets";
import AlertRules from "@/pages/alert-rules";
import ApiKeysPage from "@/pages/api-keys";
import AccountPage from "@/pages/account";
import Privacy from "@/pages/privacy";
import Terms from "@/pages/terms";
import NotFound from "@/pages/not-found";

function AuthedApp() {
  return (
    <ProtectedRoute>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/" element={<Dashboard />} />
          <Route path="/brews" element={<BrewList />} />
          <Route path="/brews/new" element={<BrewNew />} />
          <Route path="/brews/:id/report" element={<BrewReport />} />
          <Route path="/brews/:id" element={<BrewDetail />} />
          <Route path="/hydrometers" element={<HydrometerList />} />
          <Route path="/devices/setup" element={<DeviceSetup />} />
          <Route path="/alert-targets" element={<AlertTargets />} />
          <Route path="/alert-rules" element={<AlertRules />} />
          <Route path="/settings/api-keys" element={<ApiKeysPage />} />
          <Route path="/settings/account" element={<AccountPage />} />
          <Route path="*" element={<NotFound />} />
        </Route>
      </Routes>
    </ProtectedRoute>
  );
}

export default function App() {
  return (
    <Routes>
      {/* Public: readable without an account, so the policies can be linked from sign-up. */}
      <Route path="/reports/:token" element={<PublicBrewReport />} />
      <Route path="/privacy" element={<Privacy />} />
      <Route path="/terms" element={<Terms />} />
      <Route path="*" element={<AuthedApp />} />
    </Routes>
  );
}
