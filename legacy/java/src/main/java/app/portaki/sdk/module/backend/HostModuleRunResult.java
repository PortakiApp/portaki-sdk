package app.portaki.sdk.module.backend;

/**
 * Résultat générique d'une action backend (compteurs agrégés + résumé lisible humain / logs).
 *
 * <p>{@code updatedPlainConfigJson} : JSON de configuration module en clair après action, si le backend
 * renvoie un état à persister côté hôte. {@code null} si la config stockée ne change pas.
 */
public record HostModuleRunResult(
        boolean ok,
        int succeeded,
        int failed,
        int itemsTotal,
        String summary,
        String updatedPlainConfigJson
) {

    public HostModuleRunResult(boolean ok, int succeeded, int failed, int itemsTotal, String summary) {
        this(ok, succeeded, failed, itemsTotal, summary, null);
    }

    public static HostModuleRunResult emptySuccess() {
        return new HostModuleRunResult(true, 0, 0, 0, "", null);
    }
}
