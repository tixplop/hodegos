import { Emprunt, Objet } from "../modele";

export function reserver(
  objet: Objet, debut: Date, fin: Date, emprunts: Emprunt[]
) {
  const dejaPris = emprunts.find(
    (e) => e.objetId === objet.id && e.debut === debut
  );
  if (dejaPris) {
    throw new Error("Objet indisponible");
  }
  return { objetId: objet.id, debut, fin };
}

// TODO(vous) : refuser une date de fin antérieure au début
