package app.portaki.sdk.module.backend;

import java.util.UUID;

/**
 * Contexte minimal transmis au backend d'un module hôte (auth et périmètre déjà validés par l'API).
 */
public record ModuleHostContext(UUID tenantId, UUID propertyId, String moduleId) {

    public ModuleHostContext {
        if (tenantId == null || propertyId == null || moduleId == null || moduleId.isBlank()) {
            throw new IllegalArgumentException("module_host_context_incomplete");
        }
    }
}
