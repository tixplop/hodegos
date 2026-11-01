"""Répartition des tâches à faire sur les jours de la semaine."""

from dataclasses import dataclass, field
from datetime import date, timedelta

from .modele import Tache


@dataclass
class JourPlanifie:
    jour: date
    taches: list[Tache] = field(default_factory=list)

    @property
    def duree(self) -> int:
        return sum(t.duree_minutes for t in self.taches)


def planifier_semaine(
    taches: list[Tache],
    debut: date,
    max_par_jour: int = 3,
    duree_max: int = 90,
) -> list[JourPlanifie]:
    """Place chaque tâche non faite sur le premier jour qui a encore de la place.

    Les tâches sont prises par échéance croissante ; celles qui ne tiennent
    nulle part dans la semaine sont laissées de côté.
    """
    jours = [JourPlanifie(debut + timedelta(days=i)) for i in range(7)]
    for tache in sorted(taches, key=lambda t: t.echeance):
        if tache.faite:
            continue
        for jour in jours:
            if _a_de_la_place(jour, tache, max_par_jour, duree_max):
                jour.taches.append(tache)
                break
    return jours


def _a_de_la_place(jour: JourPlanifie, tache: Tache, max_par_jour: int, duree_max: int) -> bool:
    return len(jour.taches) < max_par_jour and jour.duree + tache.duree_minutes <= duree_max


def non_planifiees(taches: list[Tache], semaine: list[JourPlanifie]) -> list[Tache]:
    placees = {id(t) for jour in semaine for t in jour.taches}
    return [t for t in taches if not t.faite and id(t) not in placees]
