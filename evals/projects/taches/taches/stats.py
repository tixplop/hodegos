"""Statistiques sur les tâches faites."""

import sqlite3
from pathlib import Path


def taches_faites_par_semaine(chemin_base: Path) -> dict[str, int]:
    connexion = sqlite3.connect(chemin_base)
    try:
        lignes = connexion.execute(
            "SELECT strftime('%Y-%W', faite_le) AS semaine, COUNT(*)"
            " FROM taches WHERE faite_le IS NOT NULL"
            " GROUP BY semaine ORDER BY semaine"
        ).fetchall()
    finally:
        connexion.close()
    return {semaine: nombre for semaine, nombre in lignes}


def taux_de_retard(chemin_base: Path) -> float:
    connexion = sqlite3.connect(chemin_base)
    try:
        total, en_retard = connexion.execute(
            "SELECT COUNT(*), SUM(faite_le > echeance)"
            " FROM taches WHERE faite_le IS NOT NULL"
        ).fetchone()
    finally:
        connexion.close()
    if not total:
        return 0.0
    return (en_retard or 0) / total
