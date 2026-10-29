export function resteAPayer(lignes, dejaVerse) {
  let total = 0;
  for (const ligne of lignes) {
    total += ligne.prixEuros * ligne.quantite;
  }
  return total - dejaVerse === 0 ? "payé" : "à payer";
}
