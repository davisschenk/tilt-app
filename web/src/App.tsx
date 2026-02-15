import { Routes, Route } from "react-router-dom";
import Dashboard from "@/pages/dashboard";
import BrewList from "@/pages/brew-list";
import BrewDetail from "@/pages/brew-detail";
import BrewNew from "@/pages/brew-new";
import HydrometerList from "@/pages/hydrometer-list";
import NotFound from "@/pages/not-found";

export default function App() {
  return (
    <Routes>
      <Route path="/" element={<Dashboard />} />
      <Route path="/brews" element={<BrewList />} />
      <Route path="/brews/new" element={<BrewNew />} />
      <Route path="/brews/:id" element={<BrewDetail />} />
      <Route path="/hydrometers" element={<HydrometerList />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
}
