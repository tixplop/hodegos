"""Accès à la base SQLite : seul module qui ouvre une connexion (ADR 0001)."""

import sqlite3
from datetime import datetime
from pathlib import Path

from .dates import depuis_texte, vers_texte
from .erreurs import TacheInconnue
from .modele import Recurrence, Tache

SCHEMA = """
CREATE TABLE IF NOT EXISTS taches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    titre TEXT NOT NULL,
    echeance TEXT NOT NULL,
    recurrence TEXT NOT NULL DEFAULT 'aucune',
    duree_minutes INTEGER NOT NULL DEFAULT 15,
    faite_le TEXT
);
"""


class Depot:
    def __init__(self, chemin: Path) -> None:
        self.chemin = chemin

    def _connexion(self) -> sqlite3.Connection:
        connexion = sqlite3.connect(self.chemin)
        connexion.row_factory = sqlite3.Row
        return connexion

    def initialiser(self) -> None:
        with self._connexion() as connexion:
            connexion.executescript(SCHEMA)

    def ajouter(self, tache: Tache) -> int:
        with self._connexion() as connexion:
            curseur = connexion.execute(
                "INSERT INTO taches (titre, echeance, recurrence, duree_minutes)"
                " VALUES (?, ?, ?, ?)",
                (tache.titre, vers_texte(tache.echeance),
                 tache.recurrence.value, tache.duree_minutes),
            )
            return int(curseur.lastrowid or 0)

    def lister(self, faites: bool | None = None) -> list[Tache]:
        """Tâches triées par échéance ; échéances et dates en UTC (ADR 0002).

        faites=None : toutes ; False : à faire ; True : déjà faites.
        """
        requete = "SELECT * FROM taches"
        if faites is not None:
            requete += " WHERE faite_le IS NOT NULL" if faites else " WHERE faite_le IS NULL"
        with self._connexion() as connexion:
            lignes = connexion.execute(requete + " ORDER BY echeance").fetchall()
        return [_vers_tache(ligne) for ligne in lignes]

    def obtenir(self, identifiant: int) -> Tache:
        with self._connexion() as connexion:
            ligne = connexion.execute(
                "SELECT * FROM taches WHERE id = ?", (identifiant,)
            ).fetchone()
        if ligne is None:
            raise TacheInconnue(identifiant)
        return _vers_tache(ligne)

    def marquer_faite(self, identifiant: int, quand: datetime) -> None:
        with self._connexion() as connexion:
            curseur = connexion.execute(
                "UPDATE taches SET faite_le = ? WHERE id = ?",
                (vers_texte(quand), identifiant),
            )
        if curseur.rowcount == 0:
            raise TacheInconnue(identifiant)

    def supprimer(self, identifiant: int) -> None:
        with self._connexion() as connexion:
            connexion.execute("DELETE FROM taches WHERE id = ?", (identifiant,))


def _vers_tache(ligne: sqlite3.Row) -> Tache:
    return Tache(
        titre=ligne["titre"],
        echeance=depuis_texte(ligne["echeance"]),
        recurrence=Recurrence(ligne["recurrence"]),
        duree_minutes=ligne["duree_minutes"],
        faite_le=depuis_texte(ligne["faite_le"]) if ligne["faite_le"] else None,
        id=ligne["id"],
    )
