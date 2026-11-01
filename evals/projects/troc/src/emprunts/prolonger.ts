import { Depot } from "../stockage/depot";
import { estPeriodeValide, seChevauchent } from "./validation";

export async function prolonger(depot: Depot, id: string, nouvelleFin: Date): Promise<void> {
  await depot.modifier((donnees) => {
    const emprunt = donnees.emprunts.find((e) => e.id === id);
    if (!emprunt) {
      throw new Error("Emprunt inconnu.");
    }
    if (!estPeriodeValide(emprunt.debut, nouvelleFin)) {
      throw new Error("Prolongation trop longue.");
    }
    const suite = { debut: emprunt.fin, fin: nouvelleFin };
    const gene = donnees.emprunts.some(
      (e) => e.id !== id && e.objet === emprunt.objet && !e.rendu && seChevauchent(e, suite),
    );
    if (gene) {
      throw new Error("Un voisin a réservé l'objet juste après.");
    }
    emprunt.fin = nouvelleFin;
  });
}
