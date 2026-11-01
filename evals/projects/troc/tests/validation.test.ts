import { describe, it, expect } from "vitest";
import { estPeriodeValide, seChevauchent } from "../src/emprunts/validation";
import { jour } from "./aide";

describe("estPeriodeValide", () => {
  it("accepte une période de trois jours", () => {
    expect(estPeriodeValide(jour("2026-05-04"), jour("2026-05-01"))).toBe(true);
  });

  it("refuse plus de quatorze jours", () => {
    expect(estPeriodeValide(jour("2026-05-20"), jour("2026-05-01"))).toBe(false);
  });
});

describe("seChevauchent", () => {
  it("deux périodes qui se touchent se chevauchent", () => {
    const a = { debut: jour("2026-05-01"), fin: jour("2026-05-03") };
    const b = { debut: jour("2026-05-03"), fin: jour("2026-05-05") };
    expect(seChevauchent(a, b)).toBe(true);
  });
});
