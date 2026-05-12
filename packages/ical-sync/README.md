# @portaki/module-ical-sync

Module **hôte** : synchronisation de flux iCal (lien d’export Airbnb, Booking, etc.).  
La logique de fetch et d’analyse vit dans **portaki-api** ; ce paquet expose le manifest npm et un panneau `renderHost` optionnel.

## Portaki

- Activer le module au niveau **Organisation → Modules**.
- Par logement : **Modules** → flux JSON → **Synchroniser** (API `POST .../modules/ical-sync/sync`).

## Développement

```bash
pnpm install
```

Voir `portaki.module.json` pour la fiche catalogue.
