def seuils(limites):
    filtres = []
    for limite in limites:
        filtres.append(lambda note: note >= limite)
    return filtres
