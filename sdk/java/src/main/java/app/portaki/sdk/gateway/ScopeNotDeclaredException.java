package app.portaki.sdk.gateway;

public final class ScopeNotDeclaredException extends RuntimeException {

  public ScopeNotDeclaredException(String moduleId, String scope) {
    super("scope_not_declared: " + moduleId + " / " + scope);
  }
}
