package app.portaki.sdk.gateway;

public final class ScopeNotGrantedException extends RuntimeException {

  public ScopeNotGrantedException(String moduleId, String scope) {
    super("scope_not_granted: " + moduleId + " / " + scope);
  }
}
