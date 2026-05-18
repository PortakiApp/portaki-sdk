package app.portaki.sdk.module.backend;

/**
 * Backend serveur d'un module hôte. Une implémentation par {@code moduleId} (monolithe ou microservice).
 */
public interface PortakiHostModuleBackend {

    /**
     * Identifiant stable du module (aligné sur {@code portaki.module.json}, ex. {@code weather}).
     */
    String moduleId();

    /**
     * Exécute une action nommée avec la configuration module déjà en clair (secrets déchiffrés côté API).
     *
     * @param plainConfigJson JSON de configuration du module, UTF-8
     */
    HostModuleRunResult run(ModuleHostContext ctx, HostModuleAction action, String plainConfigJson)
            throws ModuleBackendException;
}
