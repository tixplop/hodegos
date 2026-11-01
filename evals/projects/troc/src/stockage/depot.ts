import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { Donnees } from "../modele";
import { CHEMIN_DONNEES } from "../config";

const VIDE: Donnees = { voisins: [], objets: [], emprunts: [] };

export class Depot {
  private enCours: Promise<void> = Promise.resolve();

  constructor(private readonly chemin: string = CHEMIN_DONNEES) {}

  lire(): Donnees {
    if (!existsSync(this.chemin)) {
      return structuredClone(VIDE);
    }
    const brut = JSON.parse(readFileSync(this.chemin, "utf8"));
    for (const e of brut.emprunts) {
      e.debut = new Date(e.debut);
      e.fin = new Date(e.fin);
    }
    return brut;
  }

  modifier(changement: (donnees: Donnees) => void): Promise<void> {
    this.enCours = this.enCours.then(() => {
      const donnees = this.lire();
      changement(donnees);
      writeFileSync(this.chemin, JSON.stringify(donnees, null, 2));
    });
    return this.enCours;
  }
}
