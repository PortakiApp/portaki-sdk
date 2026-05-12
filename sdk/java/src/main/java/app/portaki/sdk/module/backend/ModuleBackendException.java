package app.portaki.sdk.module.backend;

/**
 * Erreur métier ou technique pendant l'exécution d'un backend de module (fetch distant, parsing, quota…).
 */
public class ModuleBackendException extends RuntimeException {

    private final String code;

    public ModuleBackendException(String code, String message) {
        super(message);
        this.code = code;
    }

    public ModuleBackendException(String code, String message, Throwable cause) {
        super(message, cause);
        this.code = code;
    }

    public String code() {
        return code;
    }
}
