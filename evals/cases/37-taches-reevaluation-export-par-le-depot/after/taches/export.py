"""Export des tâches au format CSV."""

import csv
from typing import TextIO

from .dates import vers_texte
from .depot import Depot

COLONNES = ["id", "titre", "echeance", "recurrence", "duree_minutes", "faite_le"]


def exporter_csv(depot: Depot, sortie: TextIO) -> int:
    taches = depot.lister()
    ecrivain = csv.writer(sortie)
    ecrivain.writerow(COLONNES)
    for tache in taches:
        ecrivain.writerow([
            tache.id,
            tache.titre,
            vers_texte(tache.echeance),
            tache.recurrence.value,
            tache.duree_minutes,
            vers_texte(tache.faite_le) if tache.faite_le else "",
        ])
    return len(taches)
