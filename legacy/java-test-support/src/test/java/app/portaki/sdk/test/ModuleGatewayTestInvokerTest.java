package app.portaki.sdk.test;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.Map;
import java.util.UUID;

import org.junit.jupiter.api.Test;

import app.portaki.sdk.gateway.GatewayModuleContext;
import app.portaki.sdk.gateway.PortakiQueryHandler;
import app.portaki.sdk.module.PortakiModule;

class ModuleGatewayTestInvokerTest {

    @Test
    void invokeQuery_returnsHandlerResult() {
        DemoHandler handler = new DemoHandler();
        GatewayModuleContext ctx =
                new GatewayModuleContextBuilder()
                        .moduleId("demo")
                        .propertyId(UUID.randomUUID().toString())
                        .scopes("property:read")
                        .scopeValidation(
                                GatewayModuleContextBuilder.scopeValidationFor(
                                        "demo", java.util.List.of("property:read"), java.util.List.of()))
                        .build();

        @SuppressWarnings("unchecked")
        Map<String, Object> result =
                (Map<String, Object>)
                        ModuleGatewayTestInvoker.invokeQuery(
                                handler, "demo.echo", Map.of("message", "hi"), ctx);

        assertEquals("hi", result.get("echo"));
    }

    @PortakiModule("demo")
    static final class DemoHandler {

        @PortakiQueryHandler(value = "demo.echo", scope = "property:read")
        Map<String, Object> echo(Map<String, Object> params, GatewayModuleContext ctx) {
            return Map.of("echo", params.get("message"), "propertyId", ctx.propertyId());
        }
    }
}
