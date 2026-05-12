package app.portaki.sdk.module.backend;

/**
 * Action callable sur le backend d'un module (sync planifiée, bouton « Synchroniser », webhook, …).
 */
public record HostModuleAction(String value) {

    public static final HostModuleAction SYNC = new HostModuleAction("sync");

    public HostModuleAction {
        if (value == null || value.isBlank()) {
            throw new IllegalArgumentException("host_module_action_blank");
        }
    }
}
