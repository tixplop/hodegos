import { Donnees } from "../modele";

/** Ajoute les champs apparus depuis la première version du fichier. */
export function migrer(donnees: Donnees): Donnees {
  for (const emprunt of donnees.emprunts) {
    if (emprunt.rendu === undefined) {
      emprunt.rendu = false;
    }
  }
  return donnees;
}
