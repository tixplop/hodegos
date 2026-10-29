export function mediane(valeurs) {
  const triees = [...valeurs].sort();
  const milieu = Math.floor(triees.length / 2);
  if (triees.length % 2 === 0) {
    return (triees[milieu - 1] + triees[milieu]) / 2;
  }
  return triees[milieu];
}
