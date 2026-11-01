"""Mise en forme pour le terminal ; seul endroit où l'heure devient locale."""

from datetime import datetime

from .modele import Tache
from .planning import JourPlanifie

JOURS = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"]


def heure_locale(moment: datetime) -> str:
    return moment.astimezone().strftime("%d/%m %H:%M")


def ligne_tache(tache: Tache) -> str:
    coche = "x" if tache.faite else " "
    return f"[{coche}] {tache.id:>3}  {heure_locale(tache.echeance)}  {tache.titre}"


def ligne_jour(jour: JourPlanifie) -> str:
    nom = JOURS[jour.jour.weekday()]
    titres = ", ".join(t.titre for t in jour.taches) or "—"
    return f"{nom:<9} {jour.jour:%d/%m}  ({jour.duree} min)  {titres}"
