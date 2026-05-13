package app.portaki.sdk.gateway;

import java.util.Collections;
import java.util.Map;
import java.util.Objects;
import java.util.Set;
import java.util.UUID;
import java.util.function.Consumer;

public final class GatewayModuleContext {

  private final String moduleId;
  private final String stayId;
  private final String propertyId;
  private final UUID tenantIdInternal;
  private final Set<String> scopes;
  private final Map<String, Object> config;
  private final Consumer<PortakiModuleEvent> eventSink;
  private final ScopeValidation scopeValidation;

  public GatewayModuleContext(
      String moduleId,
      String stayId,
      String propertyId,
      UUID tenantIdInternal,
      Set<String> scopes,
      Map<String, Object> config,
      Consumer<PortakiModuleEvent> eventSink,
      ScopeValidation scopeValidation) {
    this.moduleId = Objects.requireNonNull(moduleId);
    this.stayId = Objects.requireNonNull(stayId);
    this.propertyId = Objects.requireNonNull(propertyId);
    this.tenantIdInternal = Objects.requireNonNull(tenantIdInternal);
    this.scopes = Set.copyOf(scopes);
    this.config = Map.copyOf(config);
    this.eventSink = Objects.requireNonNull(eventSink);
    this.scopeValidation = Objects.requireNonNull(scopeValidation);
  }

  public String stayId() {
    return stayId;
  }

  public String propertyId() {
    return propertyId;
  }

  public Map<String, Object> config() {
    return Collections.unmodifiableMap(config);
  }

  public Set<String> scopes() {
    return scopes;
  }

  public String moduleId() {
    return moduleId;
  }

  public UUID tenantIdInternal() {
    return tenantIdInternal;
  }

  public void publish(PortakiModuleEvent event) {
    scopeValidation.assertEventAllowed(moduleId, event.name());
    eventSink.accept(event);
  }

  public void assertScope(String scope) {
    scopeValidation.assertScope(moduleId, scope, scopes);
  }
}
