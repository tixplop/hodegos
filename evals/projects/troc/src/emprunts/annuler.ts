import { Depot } from "../stockage/depot";
import { notifier } from "../notifications/interne";

export async function annuler(depot: Depot, id: string, par: string): Promise<boolean> {
  let annule = false;
  await depot.modifier((donnees) => {
    const emprunt = donnees.emprunts.find((e) => e.id === id);
    if (!emprunt || emprunt.rendu) {
      return;
    }
    const objet = donnees.objets.find((o) => o.id === emprunt.objet);
    if (emprunt.emprunteur !== par && objet?.proprietaire !== par) {
      throw new Error("Seuls l'emprunteur et le propriétaire peuvent annuler.");
    }
    if (emprunt.debut.getTime() <= Date.now()) {
      throw new Error("Un emprunt commencé se rend, il ne s'annule pas.");
    }
    donnees.emprunts = donnees.emprunts.filter((e) => e.id !== id);
    annule = true;
    if (objet && objet.proprietaire !== par) {
      notifier(objet.proprietaire, "Une réservation de votre objet a été annulée.");
    }
  });
  return annule;
}
