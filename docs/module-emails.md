# E-mails transactionnels module — spécification (SDK 0.7+)

Les modules **ne envoient pas** d’e-mails eux-mêmes. Ils déclarent **quand** un envoi peut avoir lieu et fournissent **le contenu** (sujet, corps texte, bouton + action invité). **Portaki** (`portaki-api`) gère : planification, destinataire, branding tenant, template HTML, liens signés, déduplication, conformité.

---

## 1. Répartition des responsabilités

| Couche | Responsabilité |
|--------|----------------|
| **`portaki.module.json`** | Déclarer les e-mails (`id`, `trigger`, audience, prérequis). Optionnel : registre des `guestActions` pour validation. |
| **Backend module (JAR)** | Méthode `@PortakiModuleEmail` : retourne `Optional<ModuleEmailContent>` (vide = ne pas envoyer pour ce séjour). |
| **`portaki-api`** | Job planifié, éligibilité séjour/module, appel du composer module, rendu Thymeleaf, envoi SMTP, journal `t_e_module_email_delivery` (à implémenter). |
| **`portaki-web` (invité)** | Lire le paramètre `portakiAction` sur l’URL du livret et ouvrir le module / surface attendue. |

Le module **ne choisit pas** l’adresse From, le layout HTML global, ni le secret de signature — uniquement du **texte** et une **action** typée.

---

## 2. Manifeste — `emails[]`

Chaque entrée décrit un **type d’envoi** stable (même `id` que `@PortakiModuleEmail`).

```json
{
  "emails": [
    {
      "id": "reminder-day-before-arrival",
      "description": {
        "fr": "Rappel la veille du check-in pour remplir le formulaire.",
        "en": "Reminder the day before check-in to complete the form."
      },
      "audience": "guest",
      "requiresGuestEmail": true,
      "trigger": {
        "type": "relativeToCheckIn",
        "offset": "-P1D",
        "atLocalTime": "10:00"
      },
      "skipWhen": ["guest.email.missing", "stay.preArrivalCompleted"]
    }
  ],
  "guestActions": [
    {
      "id": "fill-form",
      "description": {
        "fr": "Ouvre le module pré-arrivée sur le formulaire.",
        "en": "Opens the pre-arrival module on the form."
      },
      "kind": "open-module",
      "moduleId": "pre-arrival-form"
    }
  ]
}
```

### 2.1 Champs `emails[]`

| Champ | Obligatoire | Description |
|-------|-------------|-------------|
| `id` | oui | Identifiant kebab-case, unique dans le module. |
| `description` | oui | `fr` / `en` — documentation catalogue / revue de sécurité. |
| `audience` | oui | `guest` (v1) — `host` réservé pour une phase ultérieure. |
| `requiresGuestEmail` | non (défaut `true` si `audience=guest`) | Si `true`, pas d’envoi sans e-mail invité sur le séjour. |
| `trigger` | oui | Voir § 2.2. |
| `skipWhen` | non | Prédicats **plateforme** (§ 2.3). |

### 2.2 `trigger`

| `type` | Sémantique |
|--------|------------|
| `relativeToCheckIn` | Instant = `checkInAt` + `offset` (ISO-8601 duration, ex. `-P1D` = 24 h avant). |
| `relativeToCheckOut` | Instant = `checkOutAt` + `offset`. |
| `onStayCreated` | Fenêtre autour de la création du séjour ; `offset` optionnel (ex. `PT1H`). |

Champs communs :

- **`offset`** (obligatoire sauf `onStayCreated` sans délai) — durée ISO-8601 signée.
- **`atLocalTime`** (optionnel) — heure locale du bien / tenant (`HH:mm`) pour regrouper l’envoi (ex. 10:00 la veille).

La plateforme **ne déclenche** l’appel au composer qu’une fois la fenêtre atteinte et tant que le module est **actif** sur le bien.

### 2.3 `skipWhen` (prédicats plateforme v1)

| Valeur | Effet |
|--------|--------|
| `guest.email.missing` | Pas d’e-mail invité. |
| `module.disabled` | Module désactivé sur le bien. |
| `stay.preArrivalCompleted` | Flag read model séjour (ex. pré-arrivée déjà faite). |
| `stay.cancelled` | Séjour annulé / hors fenêtre. |

Les modules **ne définissent pas** de prédicats arbitraires dans le manifeste en v1 — logique fine dans `Optional.empty()` côté `@PortakiModuleEmail`.

### 2.4 `guestActions[]` (optionnel)

Registre des **identifiants d’action** référencés par le CTA. Permet à la plateforme de valider les liens avant envoi.

| Champ | Description |
|-------|-------------|
| `id` | Identifiant stable (ex. `fill-form`). |
| `kind` | `open-module` (v1). |
| `moduleId` | Module invité à ouvrir (peut être le module courant ou un autre). |

---

## 3. Backend module — Java SDK

### 3.1 Annotation

```java
@PortakiModuleEmail("reminder-day-before-arrival")
public Optional<ModuleEmailContent> reminderDayBefore(StayEmailContext ctx) {
  if (ctx.guestEmail().isEmpty()) {
    return Optional.empty();
  }
  return Optional.of(
      ModuleEmailContent.of(
          LocalizedText.of(
              "Demain : préparez votre arrivée",
              "Tomorrow: get ready for your arrival"),
          LocalizedText.of(
              "Bonjour,\n\nPensez à nous indiquer votre heure d’arrivée…",
              "Hello,\n\nPlease share your arrival time…"),
          ModuleEmailCta.of(
              LocalizedText.of("Remplir le formulaire", "Fill in the form"),
              ModuleGuestEmailAction.openModule("pre-arrival-form", "fill-form"))));
}
```

- **`Optional.empty()`** : pas d’envoi pour ce séjour (raison métier module).
- Le corps est du **texte brut** ; la plateforme convertit les paragraphes (`\n\n`) en HTML simple dans le template.

### 3.2 Types (`app.portaki.sdk.email`)

| Type | Rôle |
|------|------|
| `LocalizedText` | Paire `fr` / `en` ; `forLocale(lang)`. |
| `ModuleEmailContent` | `subject`, `body`, `cta` optionnel. |
| `ModuleEmailCta` | Libellé bouton + `ModuleGuestEmailAction`. |
| `ModuleGuestEmailAction` | `openModule(moduleId, actionId)` — seul kind en v1. |
| `StayEmailContext` | Séjour, tenant, bien, slug invité, code d’accès, dates, e-mail, locale, config module. |
| `@PortakiModuleEmail` | Lie la méthode à un `id` du manifeste. |

### 3.3 Action invité (`open-module`)

Token de requête (plateforme) :

```text
portakiAction=open-module:pre-arrival-form:fill-form
```

- **`moduleId`** : module à mettre au premier plan dans le livret.
- **`actionId`** : sémantique **définie par le module** (ouvrir un onglet, focus formulaire, etc.) — le shell invité (`@portaki/sdk`) expose `parsePortakiEmailAction()` pour le décoder.

Le module **implémente** le comportement dans son `render()` ou via un effet au mount (ex. `actionId === 'fill-form'` → scroll vers le formulaire).

---

## 4. Plateforme (`portaki-api`) — comportement attendu (hors SDK)

Non implémenté dans cette version du SDK ; contrat pour l’équipe API :

1. **Découverte** : au chargement du registre, indexer `emails[]` par `moduleId`.
2. **Tick planifié** (ShedLock) : séjours dont `trigger` est dû ± fenêtre de grâce.
3. **Composer** : invoquer la méthode `@PortakiModuleEmail` du JAR (même classloader que gateway).
4. **Déduplication** : clé `(tenantId, stayId, moduleId, emailId)` — un seul envoi réussi.
5. **Lien bouton** : URL livret invité existante + `portakiAction` + signature HMAC courte durée (même famille que le bridge module).
6. **Template** : Thymeleaf unique `module-transactional-email.html` — variables : `subject`, `bodyHtml`, `ctaLabel`, `ctaUrl`, branding tenant.
7. **Événements** : optionnel `module.email.sent` / `module.email.skipped` pour observabilité.

---

## 5. Pont invité — TypeScript (`@portaki/sdk`)

```ts
import { parsePortakiEmailAction } from '@portaki/sdk'

const action = parsePortakiEmailAction(searchParams.get('portakiAction'))
// { kind: 'open-module', moduleId: 'pre-arrival-form', actionId: 'fill-form' }
```

Le livret applique l’action après résolution du séjour (router / `GuestAddonSlots`).

---

## 6. Exemple pilote — `pre-arrival-form`

| Élément | Valeur |
|---------|--------|
| Email `id` | `reminder-day-before-arrival` |
| Trigger | `relativeToCheckIn` + `offset: -P1D` + `atLocalTime: 10:00` |
| `skipWhen` | `guest.email.missing`, `stay.preArrivalCompleted` |
| CTA | `open-module:pre-arrival-form:fill-form` |
| Complétion | Événement gateway existant `pre-arrival.completed` → flag séjour |

---

## 7. Hors périmètre v1

- Pièces jointes, e-mails hôte, marketing, réponses SMTP module.
- HTML riche dans le corps module (texte seul).
- Chaînes de relances multiples dans un seul `id` (préférer plusieurs entrées `emails[]`).

---

## 8. Versionnement

Introduit en **`portaki-module-sdk` / `@portaki/module-sdk` 0.7.0** (minor, rétrocompatible), consolidé en **1.0.0**. Les modules déclarent `requiresHostSdk: "1.0.0"` lorsqu’ils utilisent des e-mails.
