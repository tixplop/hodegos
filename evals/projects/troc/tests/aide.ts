import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Depot } from "../src/stockage/depot";

export function depotVide(): Depot {
  return new Depot(join(mkdtempSync(join(tmpdir(), "troc-")), "troc.json"));
}

export function jour(texte: string): Date {
  return new Date(`${texte}T00:00:00Z`);
}
