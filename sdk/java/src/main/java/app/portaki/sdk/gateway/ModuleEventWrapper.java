package app.portaki.sdk.gateway;

import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.UUID;

public final class ModuleEventWrapper {

  private final String moduleId;
  private final UUID tenantId;
  private final String stayId;
  private final PortakiModuleEvent event;

  public ModuleEventWrapper(String moduleId, UUID tenantId, String stayId, PortakiModuleEvent event) {
    this.moduleId = Objects.requireNonNull(moduleId);
    this.tenantId = Objects.requireNonNull(tenantId);
    this.stayId = Objects.requireNonNull(stayId);
    this.event = Objects.requireNonNull(event);
  }

  public String moduleId() {
    return moduleId;
  }

  public UUID tenantId() {
    return tenantId;
  }

  public String stayId() {
    return stayId;
  }

  public PortakiModuleEvent event() {
    return event;
  }
}
