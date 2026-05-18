package app.portaki.sdk.email;

/**
 * Action exécutée lorsque l’invité clique le bouton de l’e-mail. Encodée dans l’URL du livret sous
 * {@code portakiAction} (voir {@link #toQueryValue()}).
 */
public sealed interface ModuleGuestEmailAction permits ModuleGuestEmailAction.OpenModule {

    String KIND_OPEN_MODULE = "open-module";

    /** Ouvre le module invité {@code moduleId} avec la sémantique {@code actionId} côté UI module. */
    record OpenModule(String moduleId, String actionId) implements ModuleGuestEmailAction {

        public static final String DEFAULT_ACTION_ID = "default";

        public OpenModule {
            if (moduleId == null || moduleId.isBlank()) {
                throw new IllegalArgumentException("open_module_module_id_blank");
            }
            if (actionId == null || actionId.isBlank()) {
                actionId = DEFAULT_ACTION_ID;
            }
        }

        public static OpenModule of(String moduleId) {
            return new OpenModule(moduleId, DEFAULT_ACTION_ID);
        }

        public static OpenModule of(String moduleId, String actionId) {
            return new OpenModule(moduleId, actionId);
        }
    }

    static OpenModule openModule(String moduleId) {
        return OpenModule.of(moduleId);
    }

    static OpenModule openModule(String moduleId, String actionId) {
        return OpenModule.of(moduleId, actionId);
    }

    /**
     * Valeur du paramètre {@code portakiAction} (sans encodage URL).
     *
     * <p>Format : {@code open-module:<moduleId>:<actionId>}
     */
    default String toQueryValue() {
        if (this instanceof OpenModule open) {
            return KIND_OPEN_MODULE + ":" + open.moduleId() + ":" + open.actionId();
        }
        throw new IllegalStateException("unsupported_module_guest_email_action");
    }

    /**
     * Parse une valeur {@code portakiAction} produite par {@link #toQueryValue()} ou la plateforme.
     */
    static ModuleGuestEmailAction fromQueryValue(String raw) {
        if (raw == null || raw.isBlank()) {
            throw new IllegalArgumentException("portaki_action_blank");
        }
        String[] parts = raw.split(":", 3);
        if (parts.length < 2 || !KIND_OPEN_MODULE.equals(parts[0])) {
            throw new IllegalArgumentException("portaki_action_unsupported");
        }
        String moduleId = parts[1];
        String actionId = parts.length >= 3 ? parts[2] : OpenModule.DEFAULT_ACTION_ID;
        return OpenModule.of(moduleId, actionId);
    }
}
