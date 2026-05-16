package app.portaki.sdk.test;

public final class GatewayHandlerNotFoundException extends RuntimeException {

    public GatewayHandlerNotFoundException(Class<?> handlerType, String annotation, String handlerName) {
        super(
                "gateway_handler_not_found: "
                        + handlerType.getName()
                        + " "
                        + annotation
                        + " "
                        + handlerName);
    }
}
