package app.portaki.sdk.test;

import java.lang.annotation.Annotation;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.Map;

import app.portaki.sdk.gateway.GatewayModuleContext;
import app.portaki.sdk.gateway.PortakiCommandHandler;
import app.portaki.sdk.gateway.PortakiQueryHandler;

/**
 * Invokes {@link PortakiQueryHandler} / {@link PortakiCommandHandler} methods on a module handler instance
 * without Spring or the API {@code ModuleDispatcher}.
 */
public final class ModuleGatewayTestInvoker {

    private ModuleGatewayTestInvoker() {}

    public static Object invokeQuery(
            Object handler,
            String queryName,
            Map<String, Object> params,
            GatewayModuleContext context) {
        Method method =
                findHandlerMethod(handler.getClass(), PortakiQueryHandler.class, queryName);
        return invokeHandler(method, handler, params, context);
    }

    public static void invokeCommand(
            Object handler,
            String commandName,
            Map<String, Object> params,
            GatewayModuleContext context) {
        Method method =
                findHandlerMethod(handler.getClass(), PortakiCommandHandler.class, commandName);
        invokeHandler(method, handler, params, context);
    }

    private static Method findHandlerMethod(
            Class<?> handlerType,
            Class<? extends Annotation> annotationType,
            String handlerName) {
        for (Method method : handlerType.getDeclaredMethods()) {
            Annotation annotation = method.getAnnotation(annotationType);
            if (annotation == null) {
                continue;
            }
            String value = readHandlerName(annotation);
            if (handlerName.equals(value)) {
                method.setAccessible(true);
                return method;
            }
        }
        throw new GatewayHandlerNotFoundException(handlerType, annotationType.getSimpleName(), handlerName);
    }

    private static String readHandlerName(Annotation annotation) {
        if (annotation instanceof PortakiQueryHandler queryHandler) {
            return queryHandler.value();
        }
        if (annotation instanceof PortakiCommandHandler commandHandler) {
            return commandHandler.value();
        }
        throw new IllegalArgumentException("unsupported_handler_annotation");
    }

    private static Object invokeHandler(
            Method method,
            Object handler,
            Map<String, Object> params,
            GatewayModuleContext context) {
        try {
            Object[] args = new Object[method.getParameterCount()];
            for (int i = 0; i < method.getParameterTypes().length; i++) {
                Class<?> paramType = method.getParameterTypes()[i];
                if (Map.class.isAssignableFrom(paramType)) {
                    args[i] = params;
                } else if (GatewayModuleContext.class.isAssignableFrom(paramType)) {
                    args[i] = context;
                } else {
                    throw new IllegalArgumentException(
                            "unsupported_handler_parameter: " + paramType.getName());
                }
            }
            return method.invoke(handler, args);
        } catch (InvocationTargetException ex) {
            Throwable cause = ex.getCause();
            if (cause instanceof RuntimeException runtime) {
                throw runtime;
            }
            if (cause instanceof Error error) {
                throw error;
            }
            throw new GatewayHandlerInvocationException(method, cause);
        } catch (IllegalAccessException ex) {
            throw new GatewayHandlerInvocationException(method, ex);
        }
    }
}
