"""Commandes de la ligne de commande."""

import sys
from datetime import date, timedelta

import click

from . import affichage, config, export, rappels, recurrence, stats
from .dates import lire_jour, maintenant
from .depot import Depot
from .erreurs import ErreurTaches
from .modele import Recurrence, Tache
from .planning import planifier_semaine


@click.group()
@click.pass_context
def principal(ctx: click.Context) -> None:
    reglages = config.charger()
    depot = Depot(reglages.chemin_base)
    depot.initialiser()
    ctx.obj = (reglages, depot)


@principal.command()
@click.argument("titre")
@click.option("--le", "jour", required=True, help="AAAA-MM-JJ")
@click.option("--recurrence", "rec", default="aucune",
              type=click.Choice([r.value for r in Recurrence]))
@click.option("--duree", default=15, show_default=True)
@click.pass_obj
def ajouter(obj, titre: str, jour: str, rec: str, duree: int) -> None:
    _, depot = obj
    try:
        tache = Tache(titre, lire_jour(jour), Recurrence(rec), duree)
    except ErreurTaches as erreur:
        raise click.ClickException(str(erreur)) from erreur
    click.echo(f"Tâche n° {depot.ajouter(tache)} ajoutée.")


@principal.command()
@click.option("--toutes", is_flag=True)
@click.pass_obj
def lister(obj, toutes: bool) -> None:
    _, depot = obj
    for tache in depot.lister(faites=None if toutes else False):
        click.echo(affichage.ligne_tache(tache))


@principal.command()
@click.argument("identifiant", type=int)
@click.pass_obj
def fait(obj, identifiant: int) -> None:
    _, depot = obj
    try:
        tache = depot.obtenir(identifiant)
        depot.marquer_faite(identifiant, maintenant())
    except ErreurTaches as erreur:
        raise click.ClickException(str(erreur)) from erreur
    if (suivante := recurrence.suivante(tache)) is not None:
        depot.ajouter(suivante)


@principal.command()
@click.pass_obj
def semaine(obj) -> None:
    reglages, depot = obj
    lundi = date.today() - timedelta(days=date.today().weekday())
    jours = planifier_semaine(depot.lister(faites=False), lundi,
                              reglages.max_par_jour, reglages.duree_max_par_jour)
    for jour in jours:
        click.echo(affichage.ligne_jour(jour))


@principal.command(name="rappels")
@click.pass_obj
def rappels_cmd(obj) -> None:
    _, depot = obj
    du_jour = rappels.rappels_du_jour(depot)
    click.echo(rappels.resume(du_jour))
    for tache in du_jour:
        click.echo(affichage.ligne_tache(tache))


@principal.command(name="stats")
@click.pass_obj
def stats_cmd(obj) -> None:
    reglages, _ = obj
    for sem, nombre in stats.taches_faites_par_semaine(reglages.chemin_base).items():
        click.echo(f"{sem}  {nombre}")
    click.echo(f"En retard : {stats.taux_de_retard(reglages.chemin_base):.0%}")


@principal.command()
@click.pass_obj
def exporter(obj) -> None:
    reglages, _ = obj
    export.exporter_csv(reglages.chemin_base, sys.stdout)
