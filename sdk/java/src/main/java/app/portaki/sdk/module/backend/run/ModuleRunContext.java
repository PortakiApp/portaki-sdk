package app.portaki.sdk.module.backend.run;

import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

import app.portaki.sdk.module.backend.ModuleHostContext;

/**
 * Identifiant de corrélation et métadonnées pour une exécution backend module (un « run »).
 *
 * <p>Aucun lien avec Axon, JPA ou sagas applicatives : type purement in-process pour logs, métriques
 * et enchaînement d’étapes ({@link ModuleRunPipeline}).
 */
public final class ModuleRunContext {

    private final UUID runId;
    private final UUID tenantId;
    private final UUID propertyId;
    private final String moduleId;
    private final Instant startedAt;

    public ModuleRunContext(UUID runId, UUID tenantId, UUID propertyId, String moduleId, Instant startedAt) {
        this.runId = Objects.requireNonNull(runId, "runId");
        this.tenantId = Objects.requireNonNull(tenantId, "tenantId");
        this.propertyId = Objects.requireNonNull(propertyId, "propertyId");
        this.moduleId = Objects.requireNonNull(moduleId, "moduleId");
        this.startedAt = Objects.requireNonNull(startedAt, "startedAt");
    }

    public static ModuleRunContext start(ModuleHostContext host) {
        return start(host, UUID.randomUUID());
    }

    public static ModuleRunContext start(ModuleHostContext host, UUID runId) {
        return new ModuleRunContext(runId, host.tenantId(), host.propertyId(), host.moduleId(), Instant.now());
    }

    public UUID runId() {
        return runId;
    }

    public UUID tenantId() {
        return tenantId;
    }

    public UUID propertyId() {
        return propertyId;
    }

    public String moduleId() {
        return moduleId;
    }

    public Instant startedAt() {
        return startedAt;
    }
}
