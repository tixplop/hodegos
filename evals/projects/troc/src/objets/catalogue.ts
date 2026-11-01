import { Depot } from "../stockage/depot";
import { Objet } from "../modele";

export function listerObjets(depot: Depot): Objet[] {
  return depot.lire().objets.sort((a, b) => a.nom.localeCompare(b.nom));
}

export async function ajouterObjet(depot: Depot, objet: Objet): Promise<void> {
  if (objet.nom.trim() === "") {
    throw new Error("Un objet a besoin d'un nom.");
  }
  await depot.modifier((donnees) => {
    donnees.objets.push(objet);
  });
}

export async function retirerObjet(depot: Depot, id: string): Promise<void> {
  await depot.modifier((donnees) => {
    donnees.objets = donnees.objets.filter((o) => o.id !== id);
  });
}
