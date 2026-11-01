import { describe, it, expect } from "vitest";
import { depotVide, jour } from "./aide";

describe("Depot", () => {
  it("relit les dates comme des Date", async () => {
    const depot = depotVide();
    await depot.modifier((d) => {
      d.emprunts.push({ id: "e", objet: "o", emprunteur: "a", debut: jour("2026-05-01"), fin: jour("2026-05-02"), rendu: false });
    });
    expect(depot.lire().emprunts[0].debut).toBeInstanceOf(Date);
  });

  it("sérialise les écritures concurrentes", async () => {
    const depot = depotVide();
    await Promise.all([
      depot.modifier((d) => d.voisins.push({ id: "1", nom: "Alice", logement: "A1" })),
      depot.modifier((d) => d.voisins.push({ id: "2", nom: "Bob", logement: "B2" })),
    ]);
    expect(depot.lire().voisins).toHaveLength(2);
  });
});
