package app.portaki.sdk.test;

import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

import app.portaki.sdk.gateway.PortakiModuleEvent;

/**
 * Collects {@link PortakiModuleEvent} published from a {@link GatewayModuleContext} under test.
 */
public final class CapturedModuleEvents {

    private final List<PortakiModuleEvent> events = new ArrayList<>();

    public void accept(PortakiModuleEvent event) {
        events.add(event);
    }

    public List<PortakiModuleEvent> all() {
        return Collections.unmodifiableList(events);
    }

    public void clear() {
        events.clear();
    }
}
