import { Router } from "express";
import { Depot } from "../stockage/depot";
import { listerObjets, ajouterObjet } from "../objets/catalogue";
import { reserver } from "../emprunts/reserver";
import { annuler } from "../emprunts/annuler";
import { prolonger } from "../emprunts/prolonger";

export function routes(depot: Depot): Router {
  const router = Router();
  router.get("/objets", (_req, res) => res.json(listerObjets(depot)));
  router.post("/objets", async (req, res) => {
    await ajouterObjet(depot, req.body);
    res.status(201).end();
  });
  router.post("/emprunts", async (req, res) => {
    const { objet, emprunteur, debut, fin } = req.body;
    res.status(201).json(await reserver(depot, objet, emprunteur, new Date(debut), new Date(fin)));
  });
  router.delete("/emprunts/:id", async (req, res) => {
    res.json({ annule: await annuler(depot, req.params.id, String(req.query.par)) });
  });
  router.post("/emprunts/:id/prolonger", async (req, res) => {
    await prolonger(depot, req.params.id, new Date(req.body.fin));
    res.status(204).end();
  });
  return router;
}
