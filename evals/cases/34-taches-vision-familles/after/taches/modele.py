"""Objets métier."""

from dataclasses import dataclass
from datetime import datetime
from enum import Enum


class Recurrence(Enum):
    AUCUNE = "aucune"
    QUOTIDIENNE = "quotidienne"
    HEBDOMADAIRE = "hebdomadaire"
    MENSUELLE = "mensuelle"


@dataclass
class Tache:
    titre: str
    echeance: datetime
    recurrence: Recurrence = Recurrence.AUCUNE
    duree_minutes: int = 15
    priorite: int = 2
    faite_le: datetime | None = None
    id: int | None = None

    @property
    def faite(self) -> bool:
        return self.faite_le is not None

    def en_retard(self, maintenant: datetime) -> bool:
        return not self.faite and self.echeance < maintenant
