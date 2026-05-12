package app.portaki.sdk.module;

/**
 * Contexte runtime pour des handlers côté <strong>client / intégration</strong> (extensions, webhooks sortants,
 * …).
 *
 * <p>Pour l'exécution <strong>serveur hôte</strong> (sync iCal, jobs, …), préférer {@link
 * app.portaki.sdk.module.backend.ModuleHostContext} et {@link app.portaki.sdk.module.backend.PortakiHostModuleBackend}.
 */
public interface ModuleContext {
    // TODO: expose Portaki API client, tenant id, secrets resolver
}
