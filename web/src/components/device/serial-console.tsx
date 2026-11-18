import { useEffect, useRef } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export default function SerialConsole({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => {
    const el = ref.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines]);

  return (
    <Card>
      <CardHeader className="py-3">
        <CardTitle className="text-sm font-medium">Serial console</CardTitle>
      </CardHeader>
      <CardContent>
        <pre
          ref={ref}
          className="h-48 overflow-auto rounded-md bg-muted p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all"
        >
          {lines.length ? lines.join("\n") : "No output yet."}
        </pre>
      </CardContent>
    </Card>
  );
}
