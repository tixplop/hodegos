import { readFileSync } from "node:fs";
import { Depot } from "../stockage/depot";
import { Emprunt } from "../modele";

export function historiqueDe(depot: Depot, emprunteur: string): Emprunt[] {
  return depot
    .lire()
    .emprunts.filter((e) => e.emprunteur === emprunteur)
    .sort((a, b) => b.debut.getTime() - a.debut.getTime());
}

/** Emprunts des années précédentes, déplacés à part pour alléger le fichier principal. */
export function archivesDe(emprunteur: string): Emprunt[] {
  const archives: Emprunt[] = JSON.parse(readFileSync("data/archives.json", "utf8"));
  return archives.filter((e) => e.emprunteur === emprunteur);
}
