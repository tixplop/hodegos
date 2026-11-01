"""Erreurs propres à l'application."""


class ErreurTaches(Exception):
    """Erreur attendue, affichée telle quelle à l'utilisateur."""


class TacheInconnue(ErreurTaches):
    def __init__(self, identifiant: int) -> None:
        super().__init__(f"aucune tâche n° {identifiant}")
        self.identifiant = identifiant


class DateInvalide(ErreurTaches):
    def __init__(self, texte: str) -> None:
        super().__init__(f"date illisible : {texte!r} (attendu AAAA-MM-JJ)")
        self.texte = texte
