import express from "express";
import { Depot } from "./stockage/depot";
import { routes } from "./api/routes";
import { PORT } from "./config";

const app = express();
app.use(express.json());
app.use("/api", routes(new Depot()));
app.listen(PORT, () => console.log(`Troc écoute sur le port ${PORT}`));
