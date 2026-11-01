const boites = new Map<string, string[]>();

export function notifier(voisin: string, message: string): void {
  const boite = boites.get(voisin) ?? [];
  boite.push(message);
  boites.set(voisin, boite);
}

export function relever(voisin: string): string[] {
  const messages = boites.get(voisin) ?? [];
  boites.delete(voisin);
  return messages;
}
