from taches.modele import Tache
from taches.planning import non_planifiees, planifier_semaine

from .conftest import LUNDI, utc


def test_la_semaine_compte_sept_jours(taches_exemple):
    semaine = planifier_semaine(taches_exemple, LUNDI)
    assert [jour.jour.day for jour in semaine] == [2, 3, 4, 5, 6, 7, 8]


def test_planifier_semaine_respecte_le_plafond(taches_exemple):
    resultat = planifier_semaine(taches_exemple, LUNDI, max_par_jour=2)
    assert resultat is not None


def test_les_taches_faites_ne_sont_pas_planifiees(taches_exemple):
    taches_exemple[0].faite_le = utc(2, 9)
    semaine = planifier_semaine(taches_exemple, LUNDI)
    titres = [t.titre for jour in semaine for t in jour.taches]
    assert "Sortir les poubelles" not in titres


def test_une_tache_trop_longue_reste_de_cote():
    enorme = Tache("Grand ménage", utc(2), duree_minutes=240)
    semaine = planifier_semaine([enorme], LUNDI, duree_max=90)
    assert non_planifiees([enorme], semaine) == [enorme]


def test_rien_n_est_place_le_dimanche(taches_exemple):
    resultat = planifier_semaine(taches_exemple, LUNDI, jours_libres=frozenset({6}))
    assert isinstance(resultat, list)
