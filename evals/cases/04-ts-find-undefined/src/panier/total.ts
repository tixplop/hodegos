import { Produit } from "../catalogue";

export function prixDe(produits: Produit[], reference: string): number {
  const produit = produits.find((p) => p.reference === reference);
  return produit!.prix;
}
