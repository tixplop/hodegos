import { Fiche, enregistrer } from "./stockage";

export async function importerTout(fiches: Fiche[]): Promise<number> {
  let enregistrees = 0;
  fiches.forEach(async (fiche) => {
    await enregistrer(fiche);
    enregistrees += 1;
  });
  return enregistrees;
}
