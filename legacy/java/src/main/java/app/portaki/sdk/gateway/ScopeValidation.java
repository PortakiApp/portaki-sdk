package app.portaki.sdk.gateway;

import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;

public final class ScopeValidation {

  private final Map<String, ModuleManifestSnapshot> manifestsByModuleId;

  public ScopeValidation(Map<String, ModuleManifestSnapshot> manifestsByModuleId) {
    this.manifestsByModuleId = Map.copyOf(manifestsByModuleId);
  }

  /**
   * Runtime microservice: scopes were validated by portaki-api; trust granted set for this module.
   */
  public static ScopeValidation trustOrchestrator(String moduleId, Set<String> grantedScopes) {
    List<String> scopeList = List.copyOf(grantedScopes);
    ModuleManifestSnapshot snap =
        ModuleManifestSnapshot.of(moduleId, scopeList, List.of("*"), List.of(), List.of());
    return new ScopeValidation(Map.of(moduleId, snap));
  }

  public void assertScope(String moduleId, String requiredScope, Set<String> grantedScopes) {
    ModuleManifestSnapshot manifest =
        Optional.ofNullable(manifestsByModuleId.get(moduleId))
            .orElseThrow(() -> new UnknownModuleException(moduleId));
    if (!manifest.scopes().contains(requiredScope)) {
      throw new ScopeNotDeclaredException(moduleId, requiredScope);
    }
    if (!grantedScopes.contains(requiredScope)) {
      throw new ScopeNotGrantedException(moduleId, requiredScope);
    }
  }

  public void assertEventAllowed(String moduleId, String eventName) {
    ModuleManifestSnapshot manifest =
        Optional.ofNullable(manifestsByModuleId.get(moduleId))
            .orElseThrow(() -> new UnknownModuleException(moduleId));
    if (manifest.publishedEventNames().contains("*")) {
      return;
    }
    boolean declared = manifest.publishedEventNames().stream().anyMatch(n -> n.equals(eventName));
    if (!declared) {
      throw new EventNotDeclaredException(moduleId, eventName);
    }
  }

  public Optional<ModuleManifestSnapshot> manifest(String moduleId) {
    return Optional.ofNullable(manifestsByModuleId.get(moduleId));
  }
}
