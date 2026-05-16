# Guide d’utilisation — Portaki SDK

Ce document résume les concepts et les types exposés par les paquets du dépôt [`portaki-sdk`](https://github.com/portaki/portaki-sdk) : **`@portaki/module-sdk`** (`sdk/module-sdk/`), le pont invité **`@portaki/sdk`** (`sdk/guest/`), et le SDK Java (`sdk/java/`).

---

## 1. SDK JavaScript module invité (`sdk/module-sdk/` — `@portaki/module-sdk`)

### Rôle

Décrire un **module invité** : métadonnées de navigation, rendu React en fonction du séjour et de la langue, optionnellement des marqueurs carte.

### API principale

| Symbole | Rôle |
|---------|------|
| `definePortakiModule(def)` | Retourne la définition du module (pass-through typé). |
| `PortakiModuleDefinition` | Contrat : `id`, `label`, `icon`, `render`, options (`navSlot`, `mapOverlay`, `visibleOnStatus`, `mapMarkers`). |
| `PortakiRenderContext` | Données au rendu : `property`, `stay` optionnel, `lang`. |
| `PortakiGuestProperty` | Identifiant bien, code gare optionnel, checklist optionnelle. |
| `PortakiGuestStay` | Identifiant de séjour. |
| `PortakiModuleLoader` | Type du module par défaut exporté dynamiquement. |

### Contraintes

- **Peer dependency** : `react >= 18`.
- Le point d’entrée publié est `sdk/module-sdk/dist/` (compilation `tsc`).

### Intégration dans une app hôte

L’hôte charge le bundle du module (par chargement dynamique) et consomme l’export par défaut conforme à `PortakiModuleLoader`. Le contrat exact du chargeur côté application Portaki peut évoluer ; gardez les types du SDK alignés sur la version du runtime utilisée.

---

## 2. SDK Java (`sdk/java/`)

### Rôle

Fournir des **annotations** et des **types** pour les modules backend : identification du module, routage d’événements, réponses HTTP de webhook.

### API principale

| Type | Rôle |
|------|------|
| `@PortakiModule("id")` | Marque la classe comme module ; `value` = identifiant stable. |
| `@OnEvent("nom.logique")` | Méthode handler pour un type d’événement (ex. domaine métier). |
| `ModuleContext` | Contexte d’exécution (client API, tenant, secrets). **À ce jour**, l’interface est un stub ; les capacités seront exposées dans les prochaines versions. |
| `WebhookResponse` | Record simple `status` + `body` ; fabrique `ok()` pour HTTP 200. |
| `StayCreatedEvent` | Record aligné sur l’événement métier : séjour, tenant, bien, dates, email invité. |
| `@PortakiModuleEmail` | Composer le contenu d’un e-mail transactionnel (`ModuleEmailContent`, `StayEmailContext`) — envoi par la plateforme. Voir **[module-emails.md](./module-emails.md)**. |
| `ModuleGuestEmailAction` | Action du bouton CTA (`open-module:moduleId:actionId`) — décodage invité via `parsePortakiEmailAction` dans `@portaki/sdk`. |

### Build

- Java **21**, packaging **jar**, build **Maven** (`mvn verify`).

---

## 3. Cohérence front / back

- Les identifiants de module (`PortakiModuleDefinition.id` côté JS et `@PortakiModule` côté Java) doivent rester **cohérents** avec la configuration côté plateforme Portaki.
- Les noms d’événements `@OnEvent` doivent correspondre aux contrats d’émission côté domaine (ex. `stay.created` ↔ `StayCreatedEvent`).

Pour publier et consommer les artefacts (npmjs, Maven Central), suivez **[deployment.md](./deployment.md)**.
