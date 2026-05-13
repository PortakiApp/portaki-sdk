package app.portaki.sdk.module.backend;

/**
 * Résultat générique d'une action backend (compteurs agrégés + résumé lisible humain / logs).
 *
 * <p>{@code updatedPlainConfigJson} : JSON de configuration module en clair après action (ex. dernier sync,
 * résumé). {@code null} si le backend ne modifie pas la config persistée.
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
