"""Réglages lus dans l'environnement."""

import os
from dataclasses import dataclass
from pathlib import Path

CHEMIN_PAR_DEFAUT = Path.home() / ".local" / "share" / "taches" / "taches.db"


@dataclass(frozen=True)
class Config:
    chemin_base: Path
    max_par_jour: int = 3
    duree_max_par_jour: int = 90


def charger() -> Config:
    chemin = Path(os.environ.get("TACHES_BASE", CHEMIN_PAR_DEFAUT))
    max_par_jour = int(os.environ.get("TACHES_MAX_PAR_JOUR", "3"))
    duree_max = int(os.environ.get("TACHES_DUREE_MAX", "90"))
    chemin.parent.mkdir(parents=True, exist_ok=True)
    return Config(chemin, max_par_jour, duree_max)
