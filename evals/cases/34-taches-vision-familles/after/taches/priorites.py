"""Priorités : 1 urgente, 2 normale, 3 quand on a le temps."""

from datetime import datetime

from .modele import Tache

URGENTE = 1
NORMALE = 2
FACULTATIVE = 3

LIBELLES = {URGENTE: "urgente", NORMALE: "normale", FACULTATIVE: "facultative"}


def cle_de_tri(tache: Tache) -> tuple[int, datetime]:
    return (tache.priorite, tache.echeance)


def relever_en_retard(taches: list[Tache], maintenant: datetime) -> None:
    """Une tâche en retard passe en priorité urgente."""
    for tache in taches:
        if tache.en_retard(maintenant):
            tache.priorite = URGENTE


def libelle(tache: Tache) -> str:
    return LIBELLES.get(tache.priorite, "normale")
