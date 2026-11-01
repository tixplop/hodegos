from datetime import date, datetime, timezone

import pytest

from taches.depot import Depot
from taches.modele import Recurrence, Tache

LUNDI = date(2026, 11, 2)


def utc(jour: int, heure: int = 8) -> datetime:
    return datetime(2026, 11, jour, heure, 0, tzinfo=timezone.utc)


@pytest.fixture
def depot(tmp_path):
    depot = Depot(tmp_path / "taches.db")
    depot.initialiser()
    return depot


@pytest.fixture
def taches_exemple() -> list[Tache]:
    return [
        Tache("Sortir les poubelles", utc(2), Recurrence.HEBDOMADAIRE, 5),
        Tache("Passer l'aspirateur", utc(3), duree_minutes=40),
        Tache("Laver les vitres", utc(3), duree_minutes=60),
        Tache("Changer les draps", utc(4), duree_minutes=20),
        Tache("Arroser les plantes", utc(2), Recurrence.QUOTIDIENNE, 5),
        Tache("Nettoyer le frigo", utc(5), duree_minutes=30),
    ]
