from datetime import timezone

import pytest

from taches.erreurs import TacheInconnue
from taches.modele import Recurrence, Tache

from .conftest import utc


def test_ajouter_puis_obtenir(depot):
    identifiant = depot.ajouter(Tache("Vaisselle", utc(2), Recurrence.QUOTIDIENNE))
    tache = depot.obtenir(identifiant)
    assert tache.titre == "Vaisselle"
    assert tache.recurrence is Recurrence.QUOTIDIENNE
    assert tache.echeance == utc(2)


def test_les_dates_reviennent_en_utc(depot):
    identifiant = depot.ajouter(Tache("Vaisselle", utc(2)))
    assert depot.obtenir(identifiant).echeance.tzinfo == timezone.utc


def test_lister_trie_par_echeance_et_filtre(depot):
    depot.ajouter(Tache("Tard", utc(5)))
    premiere = depot.ajouter(Tache("Tôt", utc(2)))
    depot.marquer_faite(premiere, utc(2, 9))
    assert [t.titre for t in depot.lister()] == ["Tôt", "Tard"]
    assert [t.titre for t in depot.lister(faites=False)] == ["Tard"]
    assert [t.titre for t in depot.lister(faites=True)] == ["Tôt"]


def test_marquer_faite_une_tache_inconnue(depot):
    with pytest.raises(TacheInconnue):
        depot.marquer_faite(42, utc(2))


def test_supprimer(depot):
    identifiant = depot.ajouter(Tache("Vaisselle", utc(2)))
    depot.supprimer(identifiant)
    with pytest.raises(TacheInconnue):
        depot.obtenir(identifiant)
