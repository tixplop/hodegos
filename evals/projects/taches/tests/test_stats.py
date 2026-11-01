import pytest

from taches.modele import Tache
from taches.stats import taches_faites_par_semaine, taux_de_retard

from .conftest import utc


def test_aucune_tache_faite(depot):
    assert taches_faites_par_semaine(depot.chemin) == {}
    assert taux_de_retard(depot.chemin) == 0.0


def test_regroupe_par_semaine(depot):
    for jour in (2, 3, 10):
        identifiant = depot.ajouter(Tache("Vaisselle", utc(jour)))
        depot.marquer_faite(identifiant, utc(jour, 20))
    assert taches_faites_par_semaine(depot.chemin) == {"2026-44": 2, "2026-45": 1}


def test_taux_de_retard(depot):
    a_l_heure = depot.ajouter(Tache("Vitres", utc(5)))
    en_retard = depot.ajouter(Tache("Draps", utc(2)))
    depot.marquer_faite(a_l_heure, utc(4))
    depot.marquer_faite(en_retard, utc(6))
    assert taux_de_retard(depot.chemin) == pytest.approx(0.5)
