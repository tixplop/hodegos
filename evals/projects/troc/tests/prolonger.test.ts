import { describe, it, expect } from "vitest";
import { reserver } from "../src/emprunts/reserver";
import { prolonger } from "../src/emprunts/prolonger";
import { depotVide, jour } from "./aide";

describe("prolonger", () => {
  it("refuse un emprunt inconnu", async () => {
    await expect(prolonger(depotVide(), "inconnu", jour("2026-05-10"))).rejects.toThrow("inconnu");
  });

  it("refuse si un voisin a réservé juste après", async () => {
    const depot = depotVide();
    const premier = await reserver(depot, "escabeau", "alice", jour("2026-05-02"), jour("2026-05-04"));
    await reserver(depot, "escabeau", "bob", jour("2026-05-06"), jour("2026-05-08"));
    await expect(prolonger(depot, premier.id, jour("2026-05-07"))).rejects.toThrow();
  });
});
