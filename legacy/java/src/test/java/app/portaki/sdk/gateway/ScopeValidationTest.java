package app.portaki.sdk.gateway;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.util.List;
import java.util.Map;
import java.util.Set;

import org.junit.jupiter.api.Test;

class ScopeValidationTest {

  @Test
  void whenScopeDeclaredAndGranted_thenOk() {
    ScopeValidation v =
        new ScopeValidation(
            Map.of(
                "checklist",
                ModuleManifestSnapshot.of(
                    "checklist",
                    List.of("stay:read"),
                    List.of(),
                    List.of(),
                    List.of())));
    assertDoesNotThrow(() -> v.assertScope("checklist", "stay:read", Set.of("stay:read")));
  }

  @Test
  void whenScopeNotGranted_thenThrows() {
    ScopeValidation v =
        new ScopeValidation(
            Map.of(
                "checklist",
                ModuleManifestSnapshot.of(
                    "checklist",
                    List.of("stay:read"),
                    List.of(),
                    List.of(),
                    List.of())));
    assertThrows(ScopeNotGrantedException.class, () -> v.assertScope("checklist", "stay:read", Set.of()));
  }
}
