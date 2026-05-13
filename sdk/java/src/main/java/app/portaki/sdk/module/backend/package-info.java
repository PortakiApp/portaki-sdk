/**
 * Contrats d'exécution <strong>côté serveur</strong> pour les modules hôte (actions nommées, jobs, intégrations
 * externes, …) — types et SPI génériques, sans logique métier d'un module donné.
 *
 * <p>L'implémentation peut vivre dans l'API monolithique ou dans un <strong>microservice modules</strong> :
 * l'application cœur ne fait que résoudre le {@link app.portaki.sdk.module.backend.PortakiHostModuleBackend}
 * via un {@link app.portaki.sdk.module.backend.PortakiHostModuleBackendRegistry} (in-process, HTTP, file
 * d'attente, …).
 */
package app.portaki.sdk.module.backend;
