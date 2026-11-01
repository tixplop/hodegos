import { describe, it, expect } from "vitest";
import { reserver } from "../src/emprunts/reserver";
import { rendre } from "../src/emprunts/rendre";
import { depotVide, jour } from "./aide";

describe("rendre", () => {
  it("marque l'emprunt comme rendu", async () => {
    const depot = depotVide();
    const e = await reserver(depot, "raclette", "alice", jour("2026-05-02"), jour("2026-05-03"));
    await rendre(depot, e.id);
    expect(depot.lire().emprunts[0].rendu).toBe(true);
  });
});
