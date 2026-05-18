package app.portaki.sdk.gateway;

import java.util.Map;

public record PortakiModuleEvent(String name, Map<String, Object> data) {}
