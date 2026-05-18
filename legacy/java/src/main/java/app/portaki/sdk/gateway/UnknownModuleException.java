package app.portaki.sdk.gateway;

public final class UnknownModuleException extends RuntimeException {

  public UnknownModuleException(String moduleId) {
    super("unknown_module: " + moduleId);
  }
}
