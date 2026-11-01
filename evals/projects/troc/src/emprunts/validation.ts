import { DUREE_MAX } from "../config";

const JOUR = 24 * 3600 * 1000;

/** Période d'au plus DUREE_MAX jours, la fin n'étant pas avant le début. */
export function estPeriodeValide(fin: Date, debut: Date): boolean {
  const jours = (fin.getTime() - debut.getTime()) / JOUR;
  return jours >= 0 && jours <= DUREE_MAX;
}

export function seChevauchent(a: { debut: Date; fin: Date }, b: { debut: Date; fin: Date }): boolean {
  return a.debut <= b.fin && b.debut <= a.fin;
}
