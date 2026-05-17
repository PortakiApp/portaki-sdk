<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/PortakiApp/portaki-sdk/develop/docs/assets/portaki-wordmark-light.svg">
    <img src="https://portaki.app/portaki-wordmark.svg" width="142" height="39" alt="Portaki" />
  </picture>
</p>

<h1 align="center">Portaki Module SDK · Java</h1>

<p align="center">
  <code>app.portaki:portaki-module-sdk</code> · <strong>1.0.0</strong><br/>
  <sub>Annotations &amp; types pour modules backend Portaki</sub>
</p>

<p align="center">
  <a href="https://github.com/PortakiApp/portaki-sdk/tree/develop/sdk/java"><img src="https://img.shields.io/badge/source-sdk%2Fjava-181717?logo=github" alt="Source"></a>
</p>

---

## Maven

```xml
<dependency>
  <groupId>app.portaki</groupId>
  <artifactId>portaki-module-sdk</artifactId>
  <version>1.0.0</version>
</dependency>
```

Les coordonnées ci-dessus pointent vers la release **1.0.0** sur **Maven Central** — voir [docs/deployment.md](../../docs/deployment.md).

E-mails transactionnels module : [docs/module-emails.md](../../docs/module-emails.md) (`@PortakiModuleEmail`, `ModuleEmailContent`, `ModuleGuestEmailAction`).

---

## Build local

```bash
cd sdk/java
mvn verify
mvn install
```

---

## Paire npm

Le SDK **React** correspondant : **`@portaki/module-sdk`** — [README npm](../module-sdk/README.md).

---

## Licence

Voir le `pom.xml` et la politique du dépôt [portaki-sdk](https://github.com/PortakiApp/portaki-sdk).
