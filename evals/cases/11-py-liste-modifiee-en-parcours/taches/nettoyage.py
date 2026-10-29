def retirer_terminees(taches):
    for tache in taches:
        if tache["terminee"]:
            taches.remove(tache)
    return taches
