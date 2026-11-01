from datetime import datetime, timezone

from taches.modele import Recurrence, Tache
from taches.recurrence import prochaine_echeance, suivante

from .conftest import utc


def test_sans_recurrence_pas_de_suivante():
    assert prochaine_echeance(utc(2), Recurrence.AUCUNE) is None


def test_hebdomadaire_ajoute_sept_jours():
    assert prochaine_echeance(utc(2), Recurrence.HEBDOMADAIRE) == utc(9)


def test_mensuelle_garde_le_dernier_jour_du_mois():
    fin_janvier = datetime(2027, 1, 31, 8, tzinfo=timezone.utc)
    attendu = datetime(2027, 2, 28, 8, tzinfo=timezone.utc)
    assert prochaine_echeance(fin_janvier, Recurrence.MENSUELLE) == attendu


def test_mensuelle_passe_a_l_annee_suivante():
    decembre = datetime(2026, 12, 15, 8, tzinfo=timezone.utc)
    resultat = prochaine_echeance(decembre, Recurrence.MENSUELLE)
    assert (resultat.year, resultat.month, resultat.day) == (2027, 1, 15)


def test_suivante_repart_a_zero():
    faite = Tache("Poubelles", utc(2), Recurrence.HEBDOMADAIRE, faite_le=utc(2, 9), id=7)
    nouvelle = suivante(faite)
    assert nouvelle is not None
    assert (nouvelle.echeance, nouvelle.faite_le, nouvelle.id) == (utc(9), None, None)
