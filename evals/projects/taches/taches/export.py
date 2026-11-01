"""Export des tâches au format CSV."""

import csv
import sqlite3
from pathlib import Path
from typing import TextIO

COLONNES = ["id", "titre", "echeance", "recurrence", "duree_minutes", "faite_le"]


def exporter_csv(chemin_base: Path, sortie: TextIO) -> int:
    connexion = sqlite3.connect(chemin_base)
    try:
        lignes = connexion.execute(
            "SELECT id, titre, echeance, recurrence, duree_minutes, faite_le"
            " FROM taches ORDER BY echeance"
        ).fetchall()
    finally:
        connexion.close()
    ecrivain = csv.writer(sortie)
    ecrivain.writerow(COLONNES)
    ecrivain.writerows(lignes)
    return len(lignes)
