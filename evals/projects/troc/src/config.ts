export const CHEMIN_DONNEES = process.env.TROC_DONNEES ?? "data/troc.json";

export const PORT = Number(process.env.PORT ?? 3000);

/** Durée maximale d'un emprunt, en jours. */
export const DUREE_MAX = 14;
