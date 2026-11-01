import { readFileSync, writeFileSync } from "node:fs";
import { Emprunt } from "../modele";

const FILE = "data/rappels-envoyes.json";

export function rappelsAEnvoyer(emprunts: Emprunt[], aujourdhui: Date): Emprunt[] {
  const envoyes: string[] = JSON.parse(readFileSync(FILE, "utf8"));
  const demain = new Date(aujourdhui.getTime() + 24 * 3600 * 1000);
  return emprunts.filter((e) => !e.rendu && e.fin <= demain && !envoyes.includes(e.id));
}

export function envoyerRappelEmail(emprunt: Emprunt, adresse: string): void {
  console.log(`Rappel envoyé à ${adresse} pour l'emprunt ${emprunt.id}`);
  const envoyes: string[] = JSON.parse(readFileSync(FILE, "utf8"));
  envoyes.push(emprunt.id);
  writeFileSync(FILE, JSON.stringify(envoyes));
}
