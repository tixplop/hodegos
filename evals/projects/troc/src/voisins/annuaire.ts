import { Depot } from "../stockage/depot";
import { Voisin } from "../modele";

export function trouverVoisin(depot: Depot, id: string): Voisin | undefined {
  return depot.lire().voisins.find((v) => v.id === id);
}

export async function inscrireVoisin(depot: Depot, voisin: Voisin): Promise<void> {
  await depot.modifier((donnees) => {
    if (donnees.voisins.some((v) => v.logement === voisin.logement && v.nom === voisin.nom)) {
      throw new Error("Ce voisin est déjà inscrit.");
    }
    donnees.voisins.push(voisin);
  });
}
