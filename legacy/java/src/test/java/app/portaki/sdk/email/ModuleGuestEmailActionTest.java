package app.portaki.sdk.email;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class ModuleGuestEmailActionTest {

    @Test
    void whenOpenModule_thenQueryValueRoundTrips() {
        ModuleGuestEmailAction action = ModuleGuestEmailAction.openModule("pre-arrival-form", "fill-form");

        assertEquals("open-module:pre-arrival-form:fill-form", action.toQueryValue());

        ModuleGuestEmailAction parsed = ModuleGuestEmailAction.fromQueryValue(action.toQueryValue());
        ModuleGuestEmailAction.OpenModule open = assertInstanceOf(ModuleGuestEmailAction.OpenModule.class, parsed);
        assertEquals("pre-arrival-form", open.moduleId());
        assertEquals("fill-form", open.actionId());
    }

    @Test
    void whenDefaultActionId_thenOmittedSegmentUsesDefault() {
        ModuleGuestEmailAction parsed = ModuleGuestEmailAction.fromQueryValue("open-module:wifi-guest");

        ModuleGuestEmailAction.OpenModule open = assertInstanceOf(ModuleGuestEmailAction.OpenModule.class, parsed);
        assertEquals(ModuleGuestEmailAction.OpenModule.DEFAULT_ACTION_ID, open.actionId());
    }

    @Test
    void whenBlankQuery_thenThrows() {
        assertThrows(IllegalArgumentException.class, () -> ModuleGuestEmailAction.fromQueryValue(" "));
    }
}
