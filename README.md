# Portaki SDK

Bibliothèques officielles pour développer des **modules invités** Portaki : une surface **JavaScript / React** pour le rendu côté application, et une surface **Java** pour les modules backend (annotations, payloads d’événements).

[![CI](https://github.com/portaki/portaki-sdk/actions/workflows/ci.yml/badge.svg?branch=develop)](https://github.com/portaki/portaki-sdk/actions/workflows/ci.yml)

---

## Contenu du dépôt

| Paquet | Description | Technologie |
|--------|-------------|-------------|
| [`javascript/`](javascript/) | Types et helper `definePortakiModule` pour les modules UI | TypeScript, React 18+ |
| [`java/`](java/) | Annotations `@PortakiModule`, `@OnEvent`, types d’événements | Java 21, Maven |

---

## SDK JavaScript (`@portaki/module-sdk`)

Construction d’un module invité compatible avec le chargeur dynamique Portaki.

```bash
npm install @portaki/module-sdk react
```

```tsx
import { definePortakiModule } from '@portaki/module-sdk'

export default definePortakiModule({
  id: 'example',
  label: { fr: 'Exemple', en: 'Example' },
  icon: 'sparkles',
  navSlot: 'section',
  render: ({ property, stay, lang }) => (
    <section>
      <h2>{lang === 'fr' ? property.id : property.id}</h2>
    </section>
  ),
})
```

Principaux exports : `definePortakiModule`, `PortakiModuleDefinition`, `PortakiRenderContext`, `PortakiGuestProperty`, `PortakiGuestStay`, `LangCode`.

---

## SDK Java (`app.portaki:portaki-module-sdk`)

Annotations et types pour structurer un module backend Portaki.

**Coordonnées Maven** (une fois le dépôt configuré comme dépôt Maven — voir [docs/deployment.md](docs/deployment.md)) :

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>0.1.0-SNAPSHOT</version>
</dependency>
```

Exemple minimal :

```java
import app.portaki.sdk.module.ModuleContext;
import app.portaki.sdk.module.PortakiModule;
import app.portaki.sdk.module.OnEvent;
import app.portaki.sdk.event.StayCreatedEvent;

@PortakiModule("my-backend-module")
public class MyModule {

    @OnEvent("stay.created")
    public void onStayCreated(ModuleContext ctx, StayCreatedEvent event) {
        // …
    }
}
```

---

## Développement local

**JavaScript**

```bash
cd javascript
npm ci
npm run build
```

Les artefacts compilés sont dans `javascript/dist/`.

**Java**

```bash
cd java
mvn verify
```

---

## CI/CD et publication sur GitHub

- **CI** : build JS + `mvn verify` sur les branches `main` et `develop` ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)).
- **Publication automatique** : à chaque **push sur `develop`** (avec changements dans `javascript/` ou `java/`), les workflows publient vers **GitHub Packages** — npm en **`major.minor.<run>`** (ex. `0.1.42`, dérivé du `package.json` + numéro de run GitHub, sans suffixe `-develop`) et Maven en **SNAPSHOT** (`0.1.0-SNAPSHOT`). Voir [docs/deployment.md](docs/deployment.md).
- **Publication npm** (release / manuel inclus) : [`.github/workflows/publish-npm.yml`](.github/workflows/publish-npm.yml).
- **Publication Maven** : [`.github/workflows/publish-maven.yml`](.github/workflows/publish-maven.yml).

Procédure détaillée (tags de release, `.npmrc`, `settings.xml`, permissions) : **[docs/deployment.md](docs/deployment.md)**.

Guide d’utilisation des API : **[docs/getting-started.md](docs/getting-started.md)**.

---

## Nom du paquet npm et scope GitHub

Sur GitHub Packages, le **scope npm** doit correspondre au propriétaire du dépôt GitHub (compte ou organisation), en **minuscules**. Le workflow de publication définit automatiquement le nom publié sur `@<owner>/module-sdk`. Adaptez vos imports dans les applications consommatrices en conséquence (par ex. `@mon-org/module-sdk`).

Mettez à jour le champ `repository` dans [`javascript/package.json`](javascript/package.json) si l’URL du dépôt diffère de `github.com/portaki/portaki-sdk`.

---

## Licence

MIT — voir les champs `license` des paquets individuels.
