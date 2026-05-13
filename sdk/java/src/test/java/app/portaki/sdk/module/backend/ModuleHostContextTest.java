package app.portaki.sdk.module.backend;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.UUID;

import org.junit.jupiter.api.Test;

class ModuleHostContextTest {

    @Test
    void whenValidIds_thenContextCreated() {
        // Given
        UUID tenant = UUID.randomUUID();
        UUID property = UUID.randomUUID();

        // When
        ModuleHostContext ctx = new ModuleHostContext(tenant, property, "fixtures-demo");

        // Then
        assertEquals(tenant, ctx.tenantId());
        assertEquals(property, ctx.propertyId());
        assertEquals("fixtures-demo", ctx.moduleId());
    }

    @Test
    void whenModuleIdBlank_thenThrows() {
        // Given
        UUID tenant = UUID.randomUUID();
        UUID property = UUID.randomUUID();

        // When / Then
        assertThrows(IllegalArgumentException.class, () -> new ModuleHostContext(tenant, property, " "));
    }
}
