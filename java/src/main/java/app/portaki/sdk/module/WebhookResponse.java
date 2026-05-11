package app.portaki.sdk.module;

public record WebhookResponse(int status, String body) {
  public static WebhookResponse ok() {
    return new WebhookResponse(200, "");
  }
}
