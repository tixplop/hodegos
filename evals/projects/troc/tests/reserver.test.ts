import { describe, it, expect } from "vitest";
import { reserver } from "../src/emprunts/reserver";
import { depotVide, jour } from "./aide";

describe("reserver", () => {
  it("enregistre une réservation libre", async () => {
    const depot = depotVide();
    await reserver(depot, "perceuse", "alice", jour("2026-05-02"), jour("2026-05-04"));
    expect(depot.lire().emprunts).toHaveLength(1);
  });

  it("refuse un chevauchement", async () => {
    const depot = depotVide();
    await reserver(depot, "perceuse", "alice", jour("2026-05-02"), jour("2026-05-04"));
    await expect(reserver(depot, "perceuse", "bob", jour("2026-05-03"), jour("2026-05-05"))).rejects.toThrow();
  });

  it("refuse une période à l'envers", async () => {
    await expect(reserver(depotVide(), "perceuse", "alice", jour("2026-05-04"), jour("2026-05-02"))).rejects.toThrow();
  });
});
