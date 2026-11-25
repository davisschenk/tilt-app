import { Beer, Bell, Camera, FlaskConical, LineChart, Radio } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { LegalFooterLinks } from "@/components/legal-layout";
import { useAuth } from "@/lib/auth-context";

const FEATURES = [
  {
    icon: Radio,
    title: "Wireless readings",
    body: "A small ESP32 bridge picks up your Tilt hydrometer over Bluetooth and posts gravity and temperature over Wi-Fi. Flash it from your browser.",
  },
  {
    icon: LineChart,
    title: "Fermentation at a glance",
    body: "Live gravity and temperature charts, ABV, and attenuation for every brew, active or finished.",
  },
  {
    icon: Bell,
    title: "Alerts where you are",
    body: "Get notified in Discord, Slack, or any webhook when gravity plateaus or temperature leaves your range.",
  },
  {
    icon: FlaskConical,
    title: "Nutrient schedules",
    body: "Staggered nutrient addition schedules and yeast temperature safety checks for meads and wines.",
  },
  {
    icon: Camera,
    title: "Brew log with photos",
    body: "Record pitches, dry hops, racking and tasting notes against the fermentation curve.",
  },
];

export default function Landing() {
  const { login } = useAuth();

  return (
    <div className="min-h-screen bg-background">
      <header className="mx-auto flex max-w-5xl items-center justify-between px-4 py-4">
        <div className="flex items-center gap-2 font-semibold">
          <Beer className="h-6 w-6 text-primary" />
          TiltBrew
        </div>
        <Button variant="outline" onClick={login}>
          Sign in
        </Button>
      </header>

      <main className="mx-auto max-w-5xl px-4">
        <section className="py-12 text-center sm:py-20">
          <h1 className="text-3xl font-bold tracking-tight sm:text-5xl">
            Know how your fermentation is doing, without opening the lid.
          </h1>
          <p className="mx-auto mt-4 max-w-2xl text-muted-foreground sm:text-lg">
            TiltBrew tracks your Tilt hydrometer readings, charts every brew, and tells you when
            something needs attention. Each account keeps its own private brews and devices.
          </p>
          <div className="mt-8 flex flex-col items-center justify-center gap-3 sm:flex-row">
            <Button size="lg" onClick={login}>
              Sign in or create an account
            </Button>
            <p className="text-xs text-muted-foreground">
              By continuing you agree to the <a className="underline" href="/terms">terms</a> and{" "}
              <a className="underline" href="/privacy">privacy policy</a>.
            </p>
          </div>
        </section>

        <section className="grid gap-4 pb-16 sm:grid-cols-2 lg:grid-cols-3">
          {FEATURES.map(({ icon: Icon, title, body }) => (
            <Card key={title}>
              <CardHeader>
                <Icon className="mb-1 h-6 w-6 text-primary" />
                <CardTitle className="text-lg">{title}</CardTitle>
                <CardDescription>{body}</CardDescription>
              </CardHeader>
              <CardContent />
            </Card>
          ))}
        </section>
      </main>

      <footer className="mx-auto max-w-5xl px-4 pb-10">
        <LegalFooterLinks className="justify-center" />
      </footer>
    </div>
  );
}
