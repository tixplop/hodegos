import { Creneau } from "./modele";

export function prochainsCreneaux(creneaux: Creneau[], n: number): Creneau[] {
  return creneaux
    .sort((a, b) => a.debut.getTime() - b.debut.getTime())
    .slice(0, n);
}
