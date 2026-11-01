"""Rappel du matin : ce qui est à faire aujourd'hui ou déjà en retard."""

from datetime import date

from .depot import Depot
from .modele import Tache


def rappels_du_jour(depot: Depot, aujourd_hui: date | None = None) -> list[Tache]:
    jour = aujourd_hui or date.today()
    a_faire = depot.lister(faites=False)
    return [tache for tache in a_faire if tache.echeance.date() <= jour]


def resume(rappels: list[Tache]) -> str:
    if not rappels:
        return "Rien à faire aujourd'hui."
    duree = sum(tache.duree_minutes for tache in rappels)
    pluriel = "s" if len(rappels) > 1 else ""
    return f"{len(rappels)} tâche{pluriel} aujourd'hui, environ {duree} min."
