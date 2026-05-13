package app.portaki.sdk.gateway;

public final class EventNotDeclaredException extends RuntimeException {

  public EventNotDeclaredException(String moduleId, String eventName) {
    super("event_not_declared: " + moduleId + " / " + eventName);
  }
}
