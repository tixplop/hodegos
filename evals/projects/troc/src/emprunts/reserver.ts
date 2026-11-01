import { Depot } from "../stockage/depot";
import { Emprunt } from "../modele";
import { estPeriodeValide, seChevauchent } from "./validation";
import { notifier } from "../notifications/interne";

export async function reserver(
  depot: Depot,
  objet: string,
  emprunteur: string,
  debut: Date,
  fin: Date,
): Promise<Emprunt> {
  if (!estPeriodeValide(fin, debut)) {
    throw new Error("Période invalide.");
  }
  const demande: Emprunt = { id: crypto.randomUUID(), objet, emprunteur, debut, fin, rendu: false };
  await depot.modifier((donnees) => {
    const occupe = donnees.emprunts.some((e) => e.objet === objet && !e.rendu && seChevauchent(e, demande));
    if (occupe) {
      throw new Error("Objet déjà réservé sur cette période.");
    }
    donnees.emprunts.push(demande);
  });
  notifier(emprunteur, `Réservation acceptée du ${debut.toLocaleDateString()} au ${fin.toLocaleDateString()}.`);
  return demande;
}
