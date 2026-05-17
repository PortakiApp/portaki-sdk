package app.portaki.sdk.test;

import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

import app.portaki.sdk.gateway.GatewayModuleContext;
import app.portaki.sdk.gateway.ModuleManifestSnapshot;
import app.portaki.sdk.gateway.ScopeValidation;

/**
 * Fluent builder for {@link GatewayModuleContext} in module unit tests.
 */
public final class GatewayModuleContextBuilder {

    private String moduleId = "test-module";
    private String stayId = "stay-test-001";
    private String propertyId = "property-test-001";
    private UUID tenantId = UUID.fromString("00000000-0000-4000-8000-000000000001");
    private Set<String> scopes = new HashSet<>(List.of("stay:read", "property:read", "host:property:write"));
    private Map<String, Object> config = Map.of();
    private CapturedModuleEvents events = new CapturedModuleEvents();
    private ScopeValidation scopeValidation;

    public GatewayModuleContextBuilder moduleId(String moduleId) {
        this.moduleId = moduleId;
        return this;
    }

    public GatewayModuleContextBuilder stayId(String stayId) {
        this.stayId = stayId;
        return this;
    }

    public GatewayModuleContextBuilder withoutStay() {
        this.stayId = null;
        return this;
    }

    public GatewayModuleContextBuilder propertyId(String propertyId) {
        this.propertyId = propertyId;
        return this;
    }

    public GatewayModuleContextBuilder tenantId(UUID tenantId) {
        this.tenantId = tenantId;
        return this;
    }

    public GatewayModuleContextBuilder scopes(String... scopes) {
        this.scopes = new HashSet<>(List.of(scopes));
        return this;
    }

    public GatewayModuleContextBuilder config(Map<String, Object> config) {
        this.config = new HashMap<>(config);
        return this;
    }

    public GatewayModuleContextBuilder configEntry(String key, Object value) {
        Map<String, Object> next = new HashMap<>(this.config);
        next.put(key, value);
        this.config = next;
        return this;
    }

    public GatewayModuleContextBuilder capturedEvents(CapturedModuleEvents events) {
        this.events = events;
        return this;
    }

    public GatewayModuleContextBuilder scopeValidation(ScopeValidation scopeValidation) {
        this.scopeValidation = scopeValidation;
        return this;
    }

    public CapturedModuleEvents events() {
        return events;
    }

    public GatewayModuleContext build() {
        ScopeValidation validation =
                scopeValidation != null ? scopeValidation : scopeValidationFor(moduleId);
        return new GatewayModuleContext(
                moduleId,
                stayId,
                propertyId,
                tenantId,
                scopes,
                config,
                events::accept,
                validation);
    }

    /**
     * Scope validation snapshot with common module scopes for gateway handler unit tests.
     */
    public static ScopeValidation scopeValidationFor(String moduleId) {
        return scopeValidationFor(
                moduleId,
                List.of(
                        "stay:read",
                        "stay:write",
                        "property:read",
                        "host:property:read",
                        "host:property:write"),
                List.of());
    }

    public static ScopeValidation scopeValidationFor(
            String moduleId, List<String> declaredScopes, List<String> publishedEvents) {
        ModuleManifestSnapshot snapshot =
                ModuleManifestSnapshot.of(moduleId, declaredScopes, publishedEvents, List.of(), List.of());
        return new ScopeValidation(Map.of(moduleId, snapshot));
    }
}
