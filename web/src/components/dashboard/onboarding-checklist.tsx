import { useState } from "react";
import { Link } from "react-router-dom";
import { CircleCheck, Circle, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useApiKeys } from "@/hooks/use-api-keys";
import { useHydrometers } from "@/hooks/use-hydrometers";

const DISMISSED_KEY = "tiltbrew.onboarding.dismissed";

function readDismissed(): boolean {
  try {
    return localStorage.getItem(DISMISSED_KEY) === "1";
  } catch {
    return false;
  }
}

/**
 * First-run guide for new accounts: get an API key, flash the device, see a
 * reading. Every step is derived from real data, and the card disappears on its
 * own once the first reading has arrived.
 */
export default function OnboardingChecklist() {
  const [dismissed, setDismissed] = useState(readDismissed);
  const { data: keys, isLoading: keysLoading } = useApiKeys();
  const { data: hydrometers, isLoading: hydrometersLoading } = useHydrometers();

  if (dismissed || keysLoading || hydrometersLoading) return null;

  const hasKey = (keys?.length ?? 0) > 0;
  const hasReading = (hydrometers ?? []).some((h) => h.latestReading !== null);
  if (hasReading) return null;

  const steps = [
    {
      done: hasKey,
      title: "Create an API key",
      body: "Your device uses it to send readings to your account.",
      to: "/settings/api-keys",
      cta: "Create key",
    },
    {
      done: false,
      title: "Set up your device",
      body: "Flash the ESP32 bridge from your browser and enter your Wi-Fi and API key.",
      to: "/devices/setup",
      cta: "Open device setup",
    },
    {
      done: false,
      title: "See your first reading",
      body: "Put your Tilt in the fermenter. Its readings appear here within a minute or two.",
      to: null,
      cta: "",
    },
  ];

  function dismiss() {
    try {
      localStorage.setItem(DISMISSED_KEY, "1");
    } catch {
      // Not persisted (private window); it just comes back next visit.
    }
    setDismissed(true);
  }

  return (
    <Card className="mb-6">
      <CardHeader className="flex flex-row items-start justify-between space-y-0">
        <div>
          <CardTitle>Get started</CardTitle>
          <CardDescription>Three steps to your first fermentation chart.</CardDescription>
        </div>
        <Button variant="ghost" size="icon" onClick={dismiss}>
          <X className="h-4 w-4" />
          <span className="sr-only">Dismiss</span>
        </Button>
      </CardHeader>
      <CardContent>
        <ol className="space-y-4">
          {steps.map((s) => (
            <li key={s.title} className="flex items-start gap-3">
              {s.done ? (
                <CircleCheck className="mt-0.5 h-5 w-5 shrink-0 text-green-700 dark:text-green-500" />
              ) : (
                <Circle className="mt-0.5 h-5 w-5 shrink-0 text-muted-foreground" />
              )}
              <div className="flex-1">
                <p className={s.done ? "font-medium line-through text-muted-foreground" : "font-medium"}>
                  {s.title}
                </p>
                <p className="text-sm text-muted-foreground">{s.body}</p>
              </div>
              {s.to && !s.done && (
                <Button asChild size="sm" variant="outline">
                  <Link to={s.to}>{s.cta}</Link>
                </Button>
              )}
            </li>
          ))}
        </ol>
      </CardContent>
    </Card>
  );
}
