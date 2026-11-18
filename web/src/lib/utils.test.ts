import { expect, it } from "vitest"
import { cn } from "./utils"

it("merges and dedupes tailwind classes", () => {
  expect(cn("p-2", undefined, "p-4")).toBe("p-4")
})
