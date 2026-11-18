import { Link } from "react-router-dom";
import { Beer } from "lucide-react";
import type { ReactNode } from "react";

/** Contact address shown on the legal pages; set VITE_CONTACT_EMAIL when building. */
const CONTACT_EMAIL: string | undefined = import.meta.env.VITE_CONTACT_EMAIL || undefined;

export function LegalFooterLinks({ className = "" }: { className?: string }) {
  return (
    <nav className={`flex gap-4 text-sm text-muted-foreground ${className}`}>
      <Link to="/privacy" className="hover:underline">
        Privacy
      </Link>
      <Link to="/terms" className="hover:underline">
        Terms
      </Link>
    </nav>
  );
}

export default function LegalLayout({ title, updated, children }: { title: string; updated: string; children: ReactNode }) {
  return (
    <div className="min-h-screen bg-background">
      <header className="mx-auto flex max-w-3xl items-center justify-between px-4 py-4">
        <Link to="/" className="flex items-center gap-2 font-semibold">
          <Beer className="h-6 w-6 text-primary" />
          TiltBrew
        </Link>
        <LegalFooterLinks />
      </header>
      <main className="mx-auto max-w-3xl space-y-4 px-4 pb-16 [&_h2]:mt-8 [&_h2]:text-xl [&_h2]:font-semibold [&_li]:ml-5 [&_li]:list-disc [&_p]:leading-relaxed">
        <h1 className="text-3xl font-bold tracking-tight">{title}</h1>
        <p className="text-sm text-muted-foreground">Last updated {updated}</p>
        {children}
        <h2>Contact</h2>
        <p>
          {CONTACT_EMAIL ? (
            <>
              Questions about this page or your data: <a className="underline" href={`mailto:${CONTACT_EMAIL}`}>{CONTACT_EMAIL}</a>.
            </>
          ) : (
            "Questions about this page or your data: contact the person who operates this instance."
          )}
        </p>
      </main>
    </div>
  );
}
