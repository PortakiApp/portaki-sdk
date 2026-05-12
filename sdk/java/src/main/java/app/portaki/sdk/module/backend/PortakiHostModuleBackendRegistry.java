package app.portaki.sdk.module.backend;

import java.util.Optional;

/**
 * Résout le backend d'un module hôte. L'API peut enregistrer des implémentations in-process ou déléguer vers
 * un client HTTP vers un microservice « modules ».
 */
public interface PortakiHostModuleBackendRegistry {

    Optional<PortakiHostModuleBackend> get(String moduleId);
}
