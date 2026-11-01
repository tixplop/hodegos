"""Lecture et production des dates, en UTC (ADR 0002)."""

from datetime import date, datetime, time, timezone

from .erreurs import DateInvalide


def maintenant() -> datetime:
    return datetime.now(timezone.utc)


def lire_jour(texte: str, heure: time = time(8, 0)) -> datetime:
    """Jour saisi (AAAA-MM-JJ), à l'heure locale donnée, converti en UTC."""
    try:
        jour = date.fromisoformat(texte)
    except ValueError as erreur:
        raise DateInvalide(texte) from erreur
    local = datetime.combine(jour, heure).astimezone()
    return local.astimezone(timezone.utc)


def vers_texte(moment: datetime) -> str:
    return moment.astimezone(timezone.utc).isoformat()


def depuis_texte(texte: str) -> datetime:
    return datetime.fromisoformat(texte).astimezone(timezone.utc)
