import { Depot } from "../stockage/depot";
import { Emprunt } from "../modele";

export function historiqueDe(depot: Depot, emprunteur: string): Emprunt[] {
  return depot
    .lire()
    .emprunts.filter((e) => e.emprunteur === emprunteur)
    .sort((a, b) => b.debut.getTime() - a.debut.getTime());
}
