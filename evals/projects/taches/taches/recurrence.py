"""Échéance suivante d'une tâche récurrente."""

import calendar
from dataclasses import replace
from datetime import datetime, timedelta

from .modele import Recurrence, Tache


def prochaine_echeance(echeance: datetime, recurrence: Recurrence) -> datetime | None:
    if recurrence is Recurrence.AUCUNE:
        return None
    if recurrence is Recurrence.QUOTIDIENNE:
        return echeance + timedelta(days=1)
    if recurrence is Recurrence.HEBDOMADAIRE:
        return echeance + timedelta(weeks=1)
    return _mois_suivant(echeance)


def _mois_suivant(moment: datetime) -> datetime:
    annee = moment.year + moment.month // 12
    mois = moment.month % 12 + 1
    dernier_jour = calendar.monthrange(annee, mois)[1]
    return moment.replace(year=annee, month=mois, day=min(moment.day, dernier_jour))


def suivante(tache: Tache) -> Tache | None:
    """Nouvelle occurrence à créer quand une tâche récurrente est faite."""
    echeance = prochaine_echeance(tache.echeance, tache.recurrence)
    if echeance is None:
        return None
    return replace(tache, echeance=echeance, faite_le=None, id=None)
