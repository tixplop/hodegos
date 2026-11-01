"""Export des tâches au format CSV ou JSON."""

import csv
import json
import sqlite3
from pathlib import Path
from typing import TextIO

COLONNES = ["id", "titre", "echeance", "recurrence", "duree_minutes", "faite_le"]


def _lignes(chemin_base: Path) -> list[tuple]:
    connexion = sqlite3.connect(chemin_base)
    try:
        return connexion.execute(
            "SELECT id, titre, echeance, recurrence, duree_minutes, faite_le"
            " FROM taches ORDER BY echeance"
        ).fetchall()
    finally:
        connexion.close()


def exporter_csv(chemin_base: Path, sortie: TextIO) -> int:
    lignes = _lignes(chemin_base)
    ecrivain = csv.writer(sortie)
    ecrivain.writerow(COLONNES)
    ecrivain.writerows(lignes)
    return len(lignes)


def exporter_json(chemin_base: Path, sortie: TextIO) -> int:
    lignes = _lignes(chemin_base)
    json.dump([dict(zip(COLONNES, ligne)) for ligne in lignes], sortie,
              ensure_ascii=False, indent=2)
    return len(lignes)
