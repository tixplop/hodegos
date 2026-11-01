import { Depot } from "../stockage/depot";

export function periodeCorrecte(debut: Date, fin: Date): boolean {
  const jours = (fin.getTime() - debut.getTime()) / (24 * 3600 * 1000);
  return jours >= 0 && jours <= 14;
}

export function estDisponible(depot: Depot, objet: string, debut: Date, fin: Date): boolean {
  if (!periodeCorrecte(debut, fin)) {
    return false;
  }
  return !depot
    .lire()
    .emprunts.some((e) => e.objet === objet && !e.rendu && e.debut <= fin && debut <= e.fin);
}
