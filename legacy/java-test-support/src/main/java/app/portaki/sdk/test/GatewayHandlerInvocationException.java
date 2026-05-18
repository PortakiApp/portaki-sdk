package app.portaki.sdk.test;

import java.lang.reflect.Method;

public final class GatewayHandlerInvocationException extends RuntimeException {

    public GatewayHandlerInvocationException(Method method, Throwable cause) {
        super("gateway_handler_invocation_failed: " + method.getName(), cause);
    }
}
