package app.portaki.module.prearrivalform;

import app.portaki.sdk.event.StayCreatedEvent;
import app.portaki.sdk.module.ModuleContext;
import app.portaki.sdk.module.OnEvent;
import app.portaki.sdk.module.PortakiModule;
import app.portaki.sdk.module.WebhookResponse;

@PortakiModule("pre-arrival-form")
public class PreArrivalFormModule {

  @OnEvent("StayCreatedEvent")
  public WebhookResponse onStayCreated(StayCreatedEvent event, ModuleContext ctx) {
    return WebhookResponse.ok();
  }
}
