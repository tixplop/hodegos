import { describe, it, expect } from "vitest";
import { ajouterObjet, listerObjets, retirerObjet } from "../src/objets/catalogue";
import { depotVide } from "./aide";

describe("catalogue", () => {
  it("liste les objets par nom", async () => {
    const depot = depotVide();
    await ajouterObjet(depot, { id: "2", nom: "Raclette", proprietaire: "bob" });
    await ajouterObjet(depot, { id: "1", nom: "Perceuse", proprietaire: "alice" });
    expect(listerObjets(depot).map((o) => o.nom)).toEqual(["Perceuse", "Raclette"]);
  });

  it("refuse un objet sans nom", async () => {
    await expect(ajouterObjet(depotVide(), { id: "3", nom: " ", proprietaire: "bob" })).rejects.toThrow();
  });

  it("retire un objet", async () => {
    const depot = depotVide();
    await ajouterObjet(depot, { id: "1", nom: "Perceuse", proprietaire: "alice" });
    await retirerObjet(depot, "1");
    expect(listerObjets(depot)).toEqual([]);
  });
});
