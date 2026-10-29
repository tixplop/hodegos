export function brancherBoutons(boutons, afficher) {
  for (var i = 0; i < boutons.length; i++) {
    boutons[i].addEventListener("click", function () {
      afficher("Bouton " + (i + 1));
    });
  }
}
