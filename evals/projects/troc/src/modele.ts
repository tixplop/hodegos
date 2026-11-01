export interface Voisin {
  id: string;
  nom: string;
  logement: string;
}

export interface Objet {
  id: string;
  nom: string;
  proprietaire: string;
  photo?: string;
}

export interface Emprunt {
  id: string;
  objet: string;
  emprunteur: string;
  debut: Date;
  fin: Date;
  rendu: boolean;
}

export interface Donnees {
  voisins: Voisin[];
  objets: Objet[];
  emprunts: Emprunt[];
}
