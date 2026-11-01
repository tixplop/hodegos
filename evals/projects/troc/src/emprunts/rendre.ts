import { Depot } from "../stockage/depot";

export async function rendre(depot: Depot, id: string): Promise<void> {
  await depot.modifier((donnees) => {
    const emprunt = donnees.emprunts.find((e) => e.id === id);
    if (emprunt) {
      emprunt.rendu = true;
    }
  });
}
